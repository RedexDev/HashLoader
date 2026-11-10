package;

import hashloader.HashLoader;
import sys.io.File;

/**
 * Punto de entrada del mod de prueba para Terex (Heaps + HashLoader).
 *
 * Estructura modular en `src/`:
 *   - `ModMain.hx`         : Inicialización, bucle de frames y registro de logs.
 *   - `MapBounds.hx`       : Marco visual de límites del mapa y recorte (clamp) del jugador.
 *   - `PlayerSprite.hx`    : Carga del sprite estático `res/player.png` incluido en el mod.
 *   - `EnemyManager.hx`    : 3 enemigos (cubos azules) que persiguen al jugador.
 *   - `GameOverOverlay.hx` : Estado de Game Over, cartel en pantalla y reinicio en 3 segundos.
 */
class ModMain {
	static var app:Dynamic = null;
	static var s2d:Dynamic = null;
	static var camera:Dynamic = null;
	static var player:Dynamic = null;
	static var timerClass:Dynamic = null;

	static var playerSpawnX:Float = 250.0;
	static var playerSpawnY:Float = 250.0;

	static var visualsDone:Bool = false;
	static var titleDone:Bool = false;
	static var frameCount:Int = 0;
	static var errorCount:Int = 0;
	static var logBuffer:String = "";

	public static function log(msg:String) {
		var line = "[TerexBounds] " + msg;
		Sys.println(line);
		logBuffer += line + "\n";
		try {
			File.saveContent("terex_bounds_mod_log.txt", logBuffer);
		} catch (_:Dynamic) {}
	}

	public static function fail(where:String, e:Dynamic) {
		if (errorCount++ < 12) log("error en " + where + ": " + Std.string(e));
	}

	public static function main() {
		log("iniciando (API de runtime de HashLoader)");
		try {
			setup();
		} catch (e:Dynamic) {
			log("ERROR fatal: " + Std.string(e));
		}
	}

	static function setup() {
		if (!HashLoader.available()) {
			log("ERROR: el puente de HashLoader no esta disponible");
			return;
		}

		log("puente v" + HashLoader.version() + ", tipos del juego=" + HashLoader.typeCount());

		app = HashLoader.gameInstance();
		if (app == null) {
			log("ERROR: gameInstance() devolvio null");
			return;
		}

		s2d = HashLoader.scene();
		player = HashLoader.field(app, "player");
		camera = HashLoader.field(app, "camera");
		timerClass = HashLoader.resolveClass("hxd.Timer");

		if (player == null) {
			log("ERROR: Main.player no encontrado");
			return;
		}
		if (s2d == null) {
			log("ERROR: escena 2D no encontrada");
			return;
		}

		var px:Dynamic = HashLoader.field(player, "x");
		var py:Dynamic = HashLoader.field(player, "y");
		if (px != null) playerSpawnX = px;
		if (py != null) playerSpawnY = py;

		MapBounds.init(s2d, player);

		log("Main encontrada: escena " + MapBounds.worldW + "x" + MapBounds.worldH
			+ ", jugador en (" + playerSpawnX + "," + playerSpawnY + ")"
			+ ", camara=" + (camera != null));

		HashLoader.onBeforeFrame(onPreFrame);
		HashLoader.onFrame(onPostFrame);
		log("callbacks registrados");
	}

	static function getDeltaTime():Float {
		if (timerClass != null) {
			try {
				var dt:Dynamic = HashLoader.staticField(timerClass, "dt");
				if (dt != null && dt > 0.0 && dt < 0.25) return dt;
			} catch (_:Dynamic) {}
		}
		return 1.0 / 60.0;
	}

	static function onPreFrame() {
		if (!visualsDone) {
			visualsDone = true;
			try {
				MapBounds.createVisuals(camera);
			} catch (e:Dynamic) {
				fail("MapBounds.createVisuals", e);
			}
			try {
				PlayerSprite.load(player);
			} catch (e:Dynamic) {
				fail("PlayerSprite.load", e);
			}
			try {
				EnemyManager.create(camera);
			} catch (e:Dynamic) {
				fail("EnemyManager.create", e);
			}
			try {
				GameOverOverlay.createOverlay(s2d);
			} catch (e:Dynamic) {
				fail("GameOverOverlay.createOverlay", e);
			}
			try {
				if (!titleDone) {
					titleDone = true;
					var ok = HashLoader.setWindowTitle("hash loader test!");
					log("titulo cambiado: " + ok);
				}
			} catch (e:Dynamic) {
				fail("setWindowTitle", e);
			}
		}

		if (GameOverOverlay.active) {
			GameOverOverlay.freezePlayer(player, camera);
		} else {
			MapBounds.clampPlayer(player);
		}
	}

	static function onPostFrame() {
		var dt = getDeltaTime();

		if (GameOverOverlay.active) {
			GameOverOverlay.freezePlayer(player, camera);
			GameOverOverlay.update(player, camera, playerSpawnX, playerSpawnY, dt);
		} else {
			MapBounds.clampPlayer(player);
			if (EnemyManager.update(player, dt)) {
				GameOverOverlay.trigger(player, camera);
			}
		}

		frameCount++;
		if (frameCount == 1) {
			log("primer frame: jugador=(" + HashLoader.field(player, "x") + "," + HashLoader.field(player, "y") + ")");
		} else if (frameCount % 300 == 0) {
			log("frame " + frameCount + ": jugador=(" + Std.int(HashLoader.field(player, "x"))
				+ "," + Std.int(HashLoader.field(player, "y")) + ") recortes=" + MapBounds.clampedCount);
		}
	}
}
