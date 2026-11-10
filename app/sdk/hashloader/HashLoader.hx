package hashloader;

#if hl

import hl.types.ArrayObj;

/**
 * API de runtime de HashLoader para mods de HashLink.
 *
 * HashLink aísla el registro de tipos de cada módulo `.hl`: un plugin cargado
 * con `sys_load_plugin` no ve las clases del juego con `Type.resolveClass()`
 * ni con `$allTypes()`. HashLoader embarca un puente nativo (`hashloader.hdll`)
 * que localiza el módulo del juego dentro del proceso y expone sus objetos de
 * clase, de modo que un mod puede trabajar por reflexión sin llevar código
 * nativo propio.
 *
 * Ejemplo mínimo:
 *
 * ```haxe
 * class ModMain {
 *     public static function main() {
 *         if (!hashloader.HashLoader.available()) return;
 *         var main = hashloader.HashLoader.gameInstance();
 *         var scene = hashloader.HashLoader.scene();
 *         hashloader.HashLoader.setWindowTitle("mi mod");
 *         hashloader.HashLoader.onFrame(function() {
 *             // lógica por frame en el hilo principal
 *         });
 *     }
 * }
 * ```
 *
 * Todas las llamadas deben hacerse desde el hilo principal (el `main` del mod
 * ya se ejecuta ahí). Los callbacks de `onBeforeFrame` / `onFrame` también se
 * invocan en el hilo principal, antes y después del update del juego.
 */
class HashLoader {
	// --- puente nativo -------------------------------------------------------
	@:hlNative("?hashloader", "available")
	static function nativeAvailable():Bool return false;

	@:hlNative("?hashloader", "version")
	static function nativeVersion():Int return 0;

	@:hlNative("?hashloader", "type_count")
	static function nativeTypeCount():Int return -1;

	@:hlNative("?hashloader", "has_type")
	static function nativeHasType(name:hl.Bytes):Bool return false;

	@:hlNative("?hashloader", "resolve")
	static function nativeResolve(name:hl.Bytes):Dynamic return null;

	@:hlNative("?hashloader", "get_context")
	static function nativeGetContext():Dynamic return null;

	@:hlNative("?hashloader", "add_pre_cb")
	static function nativeAddPre(cb:Void->Void):Void {}

	@:hlNative("?hashloader", "add_frame_cb")
	static function nativeAddFrame(cb:Void->Void):Void {}

	// --- estado --------------------------------------------------------------
	static var _available:Null<Bool> = null;
	static var _version:Int = -1;
	static var _classCache:Map<String, Dynamic> = new Map();
	static var _missingCache:Map<String, Bool> = new Map();

	/**
	 * `true` si el puente nativo de HashLoader está cargado y operativo.
	 * Devuelve `false` (en lugar de lanzar) cuando el mod se ejecuta fuera del
	 * loader o el `.hdll` no está en el `PATH`.
	 */
	public static function available():Bool {
		if (_available != null) return _available;
		try {
			_available = nativeAvailable();
		} catch (_:Dynamic) {
			_available = false;
		}
		return _available;
	}

	/** Versión del protocolo del puente nativo (0 si no está disponible). */
	public static function version():Int {
		if (!available()) return 0;
		if (_version < 0) {
			try {
				_version = nativeVersion();
			} catch (_:Dynamic) {
				_version = 0;
			}
		}
		return _version;
	}

	/** Número de tipos del módulo del juego (-1 si el puente no lo encontró). */
	public static function typeCount():Int {
		try {
			return nativeTypeCount();
		} catch (_:Dynamic) {
			return -1;
		}
	}

	/**
	 * Resuelve una clase del juego por su nombre calificado.
	 * Ejemplo: `HashLoader.resolveClass("h2d.Bitmap")`.
	 * Cachea el resultado en memoria para que llamarlo en cada frame sea O(1).
	 */
	public static function resolveClass(className:String):Dynamic {
		if (className == null) return null;
		if (_classCache.exists(className)) return _classCache.get(className);
		if (_missingCache.exists(className)) return null;
		try {
			var cls:Dynamic = nativeResolve(bytesOf(className));
			if (cls != null) {
				_classCache.set(className, cls);
			} else if (typeCount() > 0) {
				_missingCache.set(className, true);
			}
			return cls;
		} catch (_:Dynamic) {
			return null;
		}
	}

	/** `true` si el módulo del juego contiene el tipo indicado. */
	public static function classExists(className:String):Bool {
		if (className == null) return false;
		try {
			return nativeHasType(bytesOf(className));
		} catch (_:Dynamic) {
			return false;
		}
	}

	/** Registra un callback que se ejecuta antes del update de cada frame. */
	public static function onBeforeFrame(cb:Void->Void):Bool {
		if (cb == null) return false;
		try {
			nativeAddPre(cb);
			return true;
		} catch (_:Dynamic) {
			return false;
		}
	}

	/** Registra un callback que se ejecuta después del update de cada frame. */
	public static function onFrame(cb:Void->Void):Bool {
		if (cb == null) return false;
		try {
			nativeAddFrame(cb);
			return true;
		} catch (_:Dynamic) {
			return false;
		}
	}

	/** Instancia de la clase `Main` / `hxd.App` / `lime.app.Application` del juego (contexto). */
	public static function gameInstance():Dynamic {
		try {
			var ctx = nativeGetContext();
			if (ctx != null) return ctx;
		} catch (_:Dynamic) {}

		// Fallback 1: el valor capturado por hxd.System.loopFunc (Heaps).
		try {
			var system = resolveClass("hxd.System");
			if (system != null) {
				var loop = staticField(system, "loopFunc");
				if (loop != null) return hl.Api.getClosureValue(loop);
			}
		} catch (_:Dynamic) {}

		// Fallback 2: Main.instance o lime.app.Application.current (Lime / Legacy Engine).
		try {
			var mainCls = resolveClass("Main");
			if (mainCls != null) {
				var inst = staticField(mainCls, "instance");
				if (inst != null) return inst;
			}
		} catch (_:Dynamic) {}

		try {
			var appCls = resolveClass("lime.app.Application");
			if (appCls != null) {
				var cur = staticField(appCls, "current");
				if (cur != null) return cur;
			}
		} catch (_:Dynamic) {}

		return null;
	}

	/** Escena 2D del juego (`hxd.App.s2d` / `scene` en Heaps, o `FlxG.state` en HaxeFlixel), si existe. */
	public static function scene():Dynamic {
		var app = gameInstance();
		if (app != null) {
			var s = field(app, "s2d");
			if (s == null) s = field(app, "scene");
			if (s == null) s = field(app, "s3d");
			if (s != null) return s;
		}
		var state = flxState();
		if (state != null) return state;
		return null;
	}

	/** Clase `flixel.FlxG` del juego (HaxeFlixel), si existe. */
	public static function flxG():Dynamic {
		return resolveClass("flixel.FlxG");
	}

	/** Instancia activa de `flixel.FlxGame` (`FlxG.game`), si existe. */
	public static function flxGame():Dynamic {
		var g = flxG();
		return g != null ? staticField(g, "game") : null;
	}

	/** Estado activo de HaxeFlixel (`FlxG.game._state` / `FlxG.state`), si existe. */
	public static function flxState():Dynamic {
		var game = flxGame();
		if (game == null) return null;
		var st = field(game, "_state");
		if (st == null) st = call(game, "get_state");
		return st;
	}

	/** Ventana del juego (`sdl.Window`, `hxd.Window` o `lime.ui.Window`), si existe. */
	public static function window():Dynamic {
		var winCls = resolveClass("hxd.Window");
		if (winCls != null) {
			var inst = staticField(winCls, "inst");
			if (inst != null) return inst;
		}
		var sdlCls = resolveClass("sdl.Window");
		if (sdlCls != null) {
			var wins:Dynamic = staticField(sdlCls, "windows");
			if (wins != null && wins.length > 0) return wins[0];
		}
		var gsCls = resolveClass("GameState");
		if (gsCls != null) {
			var w = staticField(gsCls, "window");
			if (w != null) return w;
		}
		var appCls = resolveClass("lime.app.Application");
		if (appCls != null) {
			var cur = staticField(appCls, "current");
			if (cur != null) {
				var w = field(cur, "__window");
				if (w == null) w = field(cur, "window");
				if (w != null) return w;
			}
		}
		var app = gameInstance();
		if (app != null) {
			var w = field(app, "window");
			if (w == null) w = field(app, "__window");
			if (w != null) return w;
		}
		return null;
	}

	/**
	 * Cambia el título de la ventana. Prueba las rutas habituales de
	 * Heaps/SDL (`hxd.Window.inst.set_title`, `sdl.Window.windows[0]`).
	 */
	public static function setWindowTitle(text:String):Bool {
		if (text == null) return false;

		var winCls = resolveClass("hxd.Window");
		if (winCls != null) {
			var inst = staticField(winCls, "inst");
			if (inst != null && callSetter(inst, text)) return true;
		}

		var sdlCls = resolveClass("sdl.Window");
		if (sdlCls != null) {
			var wins:Dynamic = staticField(sdlCls, "windows");
			if (wins != null && wins.length > 0 && callSetter(wins[0], text)) return true;
			var f = staticField(sdlCls, "setTitle");
			if (f != null) {
				try {
					Reflect.callMethod(sdlCls, f, [text]);
					return true;
				} catch (_:Dynamic) {}
			}
		}

		var win = window();
		return win != null && callSetter(win, text);
	}

	// --- reflexión -----------------------------------------------------------
	public static function staticField(cls:Dynamic, name:String):Dynamic {
		if (cls == null || name == null) return null;
		try {
			return Reflect.field(cls, name);
		} catch (_:Dynamic) {
			return null;
		}
	}

	public static function setStaticField(cls:Dynamic, name:String, value:Dynamic):Bool {
		if (cls == null || name == null) return false;
		try {
			Reflect.setField(cls, name, value);
			return true;
		} catch (_:Dynamic) {
			return false;
		}
	}

	public static function field(obj:Dynamic, name:String):Dynamic {
		if (obj == null || name == null) return null;
		try {
			return Reflect.field(obj, name);
		} catch (_:Dynamic) {
			return null;
		}
	}

	public static function setField(obj:Dynamic, name:String, value:Dynamic):Bool {
		if (obj == null || name == null) return false;
		try {
			Reflect.setField(obj, name, value);
			return true;
		} catch (_:Dynamic) {
			return false;
		}
	}

	/** Llama a un método de instancia del juego. */
	public static function call(obj:Dynamic, method:String, args:Array<Dynamic> = null):Dynamic {
		if (obj == null || method == null) return null;
		var f:Dynamic = field(obj, method);
		if (f == null) return null;
		return Reflect.callMethod(obj, f, args == null ? [] : args);
	}

	/** Llama a un método estático de una clase del juego. */
	public static function callStatic(cls:Dynamic, method:String, args:Array<Dynamic> = null):Dynamic {
		if (cls == null || method == null) return null;
		var f:Dynamic = staticField(cls, method);
		if (f == null) return null;
		return Reflect.callMethod(cls, f, args == null ? [] : args);
	}

	/**
	 * Devuelve la longitud de un `Array` (`hl.types.ArrayObj`) perteneciente al
	 * módulo del juego sin realizar un `cast` cruzado entre módulos.
	 */
	public static function arrayLength(arr:Dynamic):Int {
		if (arr == null) return 0;
		var len:Dynamic = field(arr, "length");
		return len != null ? (len : Int) : 0;
	}

	/**
	 * Lee un elemento de un `Array` (`hl.types.ArrayObj`) del módulo del juego
	 * accediendo directamente a su `hl.NativeArray` subyacente (tipo primitivo
	 * compartido por toda la VM) para evitar fallos de `cast` entre módulos.
	 */
	public static function arrayGet(arr:Dynamic, index:Int):Dynamic {
		if (arr == null || index < 0) return null;
		try {
			var raw:hl.NativeArray<Dynamic> = field(arr, "array");
			if (raw != null) {
				return index < raw.length ? raw[index] : null;
			}
			return call(arr, "__get", [index]);
		} catch (_:Dynamic) {
			return null;
		}
	}

	/**
	 * Crea una instancia de una clase del juego.
	 *
	 * Usa el mismo camino que `new` en HashLink: reserva el objeto con el tipo
	 * real de la clase y llama al constructor sin clausura. Es más fiable que
	 * `Type.createInstance()` con clases de otro módulo.
	 */
	public static function newInstance(cls:Dynamic, args:Array<Dynamic> = null):Dynamic {
		if (cls == null) return null;
		if (args == null) args = [];

		var t:hl.Type = cast Reflect.field(cls, "__type__");
		if (t != null) {
			var o:Dynamic = t.allocObject();
			var ctor:Dynamic = Reflect.field(cls, "__constructor__");
			if (ctor != null) {
				var fn:Dynamic = hl.Api.noClosure(ctor);
				var a = args.copy();
				a.unshift(o);
				Reflect.callMethod(null, fn, a);
			}
			return o;
		}

		try {
			return Type.createInstance(cls, args);
		} catch (_:Dynamic) {
			return null;
		}
	}

	// --- utilidades ----------------------------------------------------------
	static function callSetter(win:Dynamic, text:String):Bool {
		var f:Dynamic = field(win, "set_title");
		if (f == null) f = field(win, "setTitle");
		if (f == null) return false;
		try {
			Reflect.callMethod(win, f, [text]);
			return true;
		} catch (_:Dynamic) {
			return false;
		}
	}

	static inline function bytesOf(s:String):hl.Bytes {
		return @:privateAccess s.bytes;
	}
}

#else

/**
 * Stubs para que el SDK pueda analizarse en otros targets. Los mods de
 * HashLoader se compilan siempre para HashLink (`-hl`).
 */
class HashLoader {
	public static function available():Bool return false;
	public static function version():Int return 0;
	public static function typeCount():Int return -1;
	public static function resolveClass(_name:String):Dynamic return null;
	public static function classExists(_name:String):Bool return false;
	public static function onBeforeFrame(_cb:Void->Void):Bool return false;
	public static function onFrame(_cb:Void->Void):Bool return false;
	public static function gameInstance():Dynamic return null;
	public static function scene():Dynamic return null;
	public static function flxG():Dynamic return null;
	public static function flxGame():Dynamic return null;
	public static function flxState():Dynamic return null;
	public static function window():Dynamic return null;
	public static function setWindowTitle(_text:String):Bool return false;
	public static function staticField(_cls:Dynamic, _name:String):Dynamic return null;
	public static function setStaticField(_cls:Dynamic, _name:String, _value:Dynamic):Bool return false;
	public static function field(_obj:Dynamic, _name:String):Dynamic return null;
	public static function setField(_obj:Dynamic, _name:String, _value:Dynamic):Bool return false;
	public static function call(_obj:Dynamic, _method:String, ?_args:Array<Dynamic>):Dynamic return null;
	public static function callStatic(_cls:Dynamic, _method:String, ?_args:Array<Dynamic>):Dynamic return null;
	public static function arrayLength(_arr:Dynamic):Int return 0;
	public static function arrayGet(_arr:Dynamic, _index:Int):Dynamic return null;
	public static function newInstance(_cls:Dynamic, ?_args:Array<Dynamic>):Dynamic return null;
}

#end
