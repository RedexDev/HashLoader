package;

import haxe.Json;
import sys.FileSystem;
import sys.io.File;
import sys.thread.Thread;

typedef ModEntry = {
	var id:String;
	var name:String;
	var version:String;
	var author:String;
	var path:String;
	var priority:Int;
}

typedef LoaderConfig = {
	var version:Int;
	var bin:String;
	var main_hl:String;
	var ?payload:String;
	var mods:Array<ModEntry>;
}

/**
 * Payload del Modding Runtime de HashLoader.
 *
 * Orden de arranque:
 *   1. Se lee `loader.json` para saber qué `main.hl` y qué mods cargar.
 *   2. Un hilo observador localiza `hxd.System.loopFunc` mediante el puente
 *      nativo `hashloader.hdll` y lo envuelve.
 *   3. El hilo principal carga el juego (`main.hl`), que entra en su bucle.
 *   4. En la primera llamada del envoltorio (ya en el hilo principal) se
 *      cargan los mods con `sys_load_plugin` y se guarda el contexto del
 *      juego (`Main` / `hxd.App`).
 *   5. En cada frame se ejecutan los callbacks `pre` y `frame` registrados
 *      por los mods a través de `hashloader.HashLoader`.
 *
 * Cargar los mods dentro del bucle elimina la carrera del enfoque anterior
 * (hilo aparte + espera fija + `$allTypes`), que además ejecutaba código
 * Haxe fuera del hilo principal.
 */
class LoaderPayload {
	@:hlNative("std", "sys_load_plugin")
	static function sys_load_plugin(path:hl.Bytes):Bool return false;

	// --- puente nativo (solo lo usa el payload) ------------------------------
	@:hlNative("?hashloader", "init")
	static function bridgeInit(caller:hl.Type):Int return 0;

	@:hlNative("?hashloader", "type_count")
	static function bridgeTypeCount():Int return -1;

	@:hlNative("?hashloader", "phase")
	static function bridgePhase():Int return 0;

	@:hlNative("?hashloader", "resolve")
	static function bridgeResolve(name:hl.Bytes):Dynamic return null;

	@:hlNative("?hashloader", "set_context")
	static function bridgeSetContext(v:Dynamic):Void {}

	@:hlNative("?hashloader", "pre_tick")
	static function bridgePreTick():Void {}

	@:hlNative("?hashloader", "frame_tick")
	static function bridgeFrameTick():Void {}

	@:hlNative("?hashloader", "excluded_caller")
	static function bridgeExcludedCaller():Bool return false;

	// --- estado --------------------------------------------------------------
	static var mods:Array<{id:String, bytes:hl.Bytes}> = [];
	static var bridgeReady:Bool = false;
	static var modsLoaded:Bool = false;
	static var originalLoop:Dynamic = null;
	static var wrapper:Void->Void = null;
	static var limePreWrapper:Int->Void = null;
	static var limePostWrapper:Int->Void = null;
	static var systemClass:Dynamic = null;
	static var gsClass:Dynamic = null;
	static var limeAppClass:Dynamic = null;
	static var mainClass:Dynamic = null;
	static var limeApp:Dynamic = null;
	static var limeHooked:Bool = false;
	static var ticked:Bool = false;
	static var hooked:Bool = false;
	static var errorCount:Int = 0;

	static function main() {
		var cfgPath = Sys.getEnv("HASHLOADER_CONFIG");
		var mainHl:String = null;

		var args = Sys.args();
		if (args != null && args.length > 0 && args[0] != null && args[0] != "") {
			mainHl = args[0];
		}

		if (cfgPath != null && cfgPath != "" && FileSystem.exists(cfgPath)) {
			try {
				var raw = File.getContent(cfgPath);
				var cfg:LoaderConfig = Json.parse(raw);
				if ((mainHl == null || mainHl == "") && cfg.main_hl != null && cfg.main_hl != "") {
					mainHl = cfg.main_hl;
				}
				if (cfg.mods != null) {
					for (m in cfg.mods) {
						if (m != null && m.path != null && m.path != "" && FileSystem.exists(m.path)) {
							mods.push({
								id: m.id != null ? m.id : "mod",
								bytes: @:privateAccess Sys.getPath(m.path)
							});
						}
					}
				}
			} catch (e:Dynamic) {
				Sys.println("[HashLoader] Error leyendo loader.json: " + Std.string(e));
			}
		}

		if (mainHl == null || mainHl == "" || !FileSystem.exists(mainHl)) {
			mainHl = detectGameBytecode();
		}

		var mainHlBytes = @:privateAccess Sys.getPath(mainHl);

		if (mods.length == 0) {
			// Sin mods: arranque directo, sin puente ni envoltorio.
			sys_load_plugin(mainHlBytes);
			return;
		}

		// Los envoltorios se crean en el hilo principal y quedan referenciados por
		// estos estaticos; el observador los instala en hxd.System.loopFunc (Heaps)
		// o en Application.onUpdate (Lime / Legacy Engine).
		wrapper = function() {
			tick();
		};
		limePreWrapper = function(_dt:Int) {
			limePreTick();
		};
		limePostWrapper = function(_dt:Int) {
			limePostTick();
		};

		startWatcher();
		sys_load_plugin(mainHlBytes);
	}

	// --------------------------------------------------------------- observador
	static function startWatcher() {
		Thread.create(function() {
			if (!initBridge()) return;

			var attempts = 0;
			var installed = false;
			while (attempts < 800) {
				attempts++;
				try {
					if (systemClass == null && gsClass == null && limeAppClass == null && mainClass == null) {
						systemClass = bridgeResolve(strBytes("hxd.System"));
						if (systemClass == null) {
							gsClass = bridgeResolve(strBytes("GameState"));
							limeAppClass = bridgeResolve(strBytes("lime.app.Application"));
							mainClass = bridgeResolve(strBytes("Main"));
						}
					}
					if (systemClass != null) {
						var loop:Dynamic = bridgeGetLoop();
						if (loop != null) {
							if (!Reflect.compareMethods(loop, wrapper)) {
								installWrapper(loop);
								installed = true;
							}
							// hxd.App.setup() vuelve a llamar a setLoop(mainLoop)
							// despues de init(): si ya hubo un frame, el
							// envoltorio esta consolidado.
							if (ticked) return;
						}
					} else {
						if (!limeHooked) {
							if (tryInstallLimeWrapper()) {
								installed = true;
							}
						}
						if (limeHooked && ticked) return;
					}
				} catch (e:Dynamic) {
					logError("observador", e);
				}
				Sys.sleep(0.05);
			}
			if (!installed)
				Sys.println("[HashLoader] AVISO: bucle principal (hxd.System / Lime onUpdate) no aparecio");
		});
	}

	static function initBridge():Bool {
		if (bridgeReady) return true;
		var attempts = 0;
		while (attempts < 400) {
			attempts++;
			try {
				var n = bridgeInit(hl.Type.get(LoaderPayload));
				if (n > 0) {
					bridgeReady = true;
					Sys.println("[HashLoader] puente nativo: " + n
						+ " tipos de juego"
						+ (bridgeExcludedCaller() ? " (modulo propio excluido)" : ""));
					return true;
				}
			} catch (e:Dynamic) {
				Sys.println("[HashLoader] puente nativo no disponible: " + Std.string(e)
					+ " (fase " + bridgePhase() + ", intento " + attempts + ")");
				return false;
			}
			Sys.sleep(0.05);
		}
		Sys.println("[HashLoader] AVISO: no se encontro el modulo del juego");
		return false;
	}

	static function installWrapper(loop:Dynamic) {
		originalLoop = loop;
		try {
			// Usamos el propio setLoop() compilado del juego: escribir el
			// campo con Reflect.setField no actualiza el global que lee
			// mainLoop, asi que el envoltorio nunca se ejecutaria.
			var setLoop:Dynamic = Reflect.field(systemClass, "setLoop");
			if (setLoop != null)
				Reflect.callMethod(null, setLoop, [wrapper]);
			else
				Reflect.setField(systemClass, "loopFunc", wrapper);
			if (!hooked) {
				hooked = true;
				Sys.println("[HashLoader] bucle enganchado (" + bridgeTypeCount() + " tipos)");
			}
		} catch (e:Dynamic) {
			logError("enganche del bucle", e);
		}
	}

	static function bridgeGetLoop():Dynamic {
		if (systemClass == null) return null;
		try {
			var getLoop:Dynamic = Reflect.field(systemClass, "getCurrentLoop");
			if (getLoop != null)
				return Reflect.callMethod(null, getLoop, []);
		} catch (e:Dynamic) {}
		return Reflect.field(systemClass, "loopFunc");
	}

	static function tryInstallLimeWrapper():Bool {
		var app:Dynamic = null;

		// 1. Si el juego expone GameState (ej. FNF Legacy Engine), esperamos a que
		//    GameState.window exista para enganchar tras onWindowCreate().
		if (gsClass != null) {
			var win:Dynamic = Reflect.field(gsClass, "window");
			if (win == null) return false;
			app = Reflect.field(win, "application");
		}

		// 2. Fallback general para juegos basados en Lime / OpenFL / Flixel:
		//    lime.app.Application.current o Main.instance.
		if (app == null && limeAppClass != null) {
			app = Reflect.field(limeAppClass, "current");
		}
		if (app == null && mainClass != null) {
			app = Reflect.field(mainClass, "instance");
		}
		if (app == null) return false;
		if (Reflect.field(app, "__window") == null && Reflect.field(app, "window") == null) return false;

		var onUpdate:Dynamic = Reflect.field(app, "onUpdate");
		if (onUpdate == null) return false;

		var listeners:Dynamic = Reflect.field(onUpdate, "__listeners");
		if (listeners != null && listeners.length <= 0) return false;

		try {
			limeApp = app;
			onUpdate.add(limePreWrapper, false, 100000);
			onUpdate.add(limePostWrapper, false, -100000);
			limeHooked = true;
			if (!hooked) {
				hooked = true;
				Sys.println("[HashLoader] bucle enganchado (" + bridgeTypeCount() + " tipos)");
			}
			return true;
		} catch (e:Dynamic) {
			logError("enganche Lime", e);
			return false;
		}
	}

	// ------------------------------------------------------------------- frame
	static function limePreTick() {
		ticked = true;
		if (!modsLoaded) {
			modsLoaded = true;
			onFirstFrame();
		}

		try {
			bridgePreTick();
		} catch (e:Dynamic) {
			logError("pre_tick", e);
		}
	}

	static function limePostTick() {
		try {
			bridgeFrameTick();
		} catch (e:Dynamic) {
			logError("frame_tick", e);
		}
	}

	static function tick() {
		ticked = true;
		if (!modsLoaded) {
			modsLoaded = true;
			onFirstFrame();
		}

		try {
			bridgePreTick();
		} catch (e:Dynamic) {
			logError("pre_tick", e);
		}

		try {
			if (originalLoop != null) {
				Reflect.callMethod(null, originalLoop, []);
			}
		} catch (e:Dynamic) {
			logError("bucle original", e);
		}

		try {
			bridgeFrameTick();
		} catch (e:Dynamic) {
			logError("frame_tick", e);
		}
	}

	static function onFirstFrame() {
		// Contexto del juego: instancia de Application / Main (Lime) o valor
		// capturado por hxd.System.loopFunc antes de reemplazarlo (Heaps).
		try {
			var ctx:Dynamic = limeApp;
			if (ctx == null && originalLoop != null) {
				ctx = hl.Api.getClosureValue(originalLoop);
			}
			if (ctx != null) {
				bridgeSetContext(ctx);
			}
		} catch (e:Dynamic) {
			logError("contexto del juego", e);
		}

		// Los mods se cargan en el hilo principal, dentro del bucle.
		var loaded = 0;
		var pending = mods;
		mods = [];
		for (mp in pending) {
			try {
				var ok = sys_load_plugin(mp.bytes);
				if (ok) loaded++;
				Sys.println("[HashLoader] mod '" + mp.id + "': " + (ok ? "cargado" : "fallo"));
			} catch (e:Dynamic) {
				Sys.println("[HashLoader] error cargando mod '" + mp.id + "': " + Std.string(e));
			}
		}
		Sys.println("[HashLoader] " + loaded + "/" + pending.length + " mods cargados");
	}

	// -------------------------------------------------------------------- util
	static function detectGameBytecode():String {
		if (FileSystem.exists("main.hl")) return "main.hl";
		if (FileSystem.exists("hlboot.dat")) return "hlboot.dat";
		try {
			var best:String = null;
			var bestSize:Int = -1;
			for (entry in FileSystem.readDirectory(".")) {
				if (FileSystem.isDirectory(entry)) continue;
				var lower = entry.toLowerCase();
				if (lower == "hashloader.hl" || lower == "payload.hl") continue;
				if (StringTools.endsWith(lower, ".hl") || lower == "hlboot.dat") {
					var sz = FileSystem.stat(entry).size;
					if (sz > bestSize) {
						bestSize = sz;
						best = entry;
					}
				}
			}
			if (best != null) return best;
		} catch (_:Dynamic) {}
		return "main.hl";
	}

	static function logError(where:String, e:Dynamic) {
		if (errorCount++ < 16) {
			Sys.println("[HashLoader] error en " + where + ": " + Std.string(e));
		}
	}

	static inline function strBytes(s:String):hl.Bytes {
		return @:privateAccess s.bytes;
	}
}
