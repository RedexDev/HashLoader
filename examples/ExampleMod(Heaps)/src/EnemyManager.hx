package;

import hashloader.HashLoader;

typedef EnemyCube = {
	var bmp:Dynamic;
	var x:Float;
	var y:Float;
	var speed:Float;
	var spawnX:Float;
	var spawnY:Float;
}

/**
 * Gestiona hasta un máximo de 3 enemigos (cubos azules) que persiguen al jugador
 * dentro de los límites del mapa y activan el Game Over al tocarlo.
 */
class EnemyManager {
	public static inline var MAX_ENEMIES:Int = 3;
	static inline var ENEMY_SIZE:Int = 36;
	static inline var HIT_DIST_X:Float = 32.0;
	static inline var HIT_DIST_Y:Float = 36.0;

	public static var enemies:Array<EnemyCube> = [];

	static function createBlueCubeTile():Dynamic {
		var tileCls:Dynamic = HashLoader.resolveClass("h2d.Tile");
		var bmdCls:Dynamic = HashLoader.resolveClass("hxd.BitmapData");

		if (bmdCls != null && tileCls != null) {
			try {
				var bmd:Dynamic = HashLoader.newInstance(bmdCls, [ENEMY_SIZE, ENEMY_SIZE]);
				// Borde azul oscuro y relleno azul intenso con núcleo celeste
				HashLoader.call(bmd, "fill", [0, 0, ENEMY_SIZE, ENEMY_SIZE, 0xFF0D47A1]);
				HashLoader.call(bmd, "fill", [3, 3, ENEMY_SIZE - 6, ENEMY_SIZE - 6, 0xFF1E88E5]);
				HashLoader.call(bmd, "fill", [8, 8, ENEMY_SIZE - 16, ENEMY_SIZE - 16, 0xFF42A5F5]);
				var t:Dynamic = HashLoader.callStatic(tileCls, "fromBitmap", [bmd]);
				HashLoader.call(bmd, "dispose", []);
				if (t != null) {
					var centered:Dynamic = HashLoader.call(t, "center", []);
					return centered != null ? centered : t;
				}
			} catch (_:Dynamic) {}
		}

		var fromColor:Dynamic = HashLoader.staticField(tileCls, "fromColor");
		var fallback:Dynamic = Reflect.callMethod(tileCls, fromColor, [0x1E88E5, ENEMY_SIZE, ENEMY_SIZE]);
		var centeredFallback:Dynamic = HashLoader.call(fallback, "center", []);
		return centeredFallback != null ? centeredFallback : fallback;
	}

	public static function create(camera:Dynamic) {
		var bmpCls:Dynamic = HashLoader.resolveClass("h2d.Bitmap");
		if (bmpCls == null) return;

		var enemyTile = createBlueCubeTile();
		if (enemyTile == null) return;

		// 3 puntos de aparición alejados de la posición inicial del jugador (250, 250)
		var spawns:Array<{x:Float, y:Float, speed:Float}> = [
			{x: 680.0, y: 100.0, speed: 155.0},
			{x: 680.0, y: 500.0, speed: 135.0},
			{x: 110.0, y: 510.0, speed: 120.0}
		];

		enemies = [];
		for (i in 0...MAX_ENEMIES) {
			var cfg = spawns[i];
			var bmp:Dynamic = HashLoader.newInstance(bmpCls, [enemyTile, camera]);
			if (bmp == null) continue;
			HashLoader.setField(bmp, "x", cfg.x);
			HashLoader.setField(bmp, "y", cfg.y);
			try {
				HashLoader.setField(bmp, "posChanged", true);
			} catch (_:Dynamic) {}

			enemies.push({
				bmp: bmp,
				x: cfg.x,
				y: cfg.y,
				speed: cfg.speed,
				spawnX: cfg.x,
				spawnY: cfg.y
			});
		}

		ModMain.log("enemigos creados: " + enemies.length + " cubos azules");
	}

	/**
	 * Actualiza el movimiento de persecución de los 3 cubos azules.
	 * Devuelve `true` si algún enemigo alcanza al jugador en este frame.
	 */
	public static function update(player:Dynamic, dt:Float):Bool {
		if (enemies.length == 0) return false;

		var px:Float = HashLoader.field(player, "x");
		var py:Float = HashLoader.field(player, "y");
		var halfEnemy = ENEMY_SIZE * 0.5;
		var eMinX = 14.0 + halfEnemy;
		var eMinY = 14.0 + halfEnemy;
		var eMaxX = MapBounds.worldW - 14.0 - halfEnemy;
		var eMaxY = MapBounds.worldH - 14.0 - halfEnemy;

		for (i in 0...enemies.length) {
			var e = enemies[i];
			var dx = px - e.x;
			var dy = py - e.y;
			var dist = Math.sqrt(dx * dx + dy * dy);

			var vx = 0.0;
			var vy = 0.0;
			if (dist > 0.001) {
				vx = (dx / dist) * e.speed;
				vy = (dy / dist) * e.speed;
			}

			// Ligera separación entre cubos azules para que no se solapen en uno solo
			for (j in 0...enemies.length) {
				if (i == j) continue;
				var other = enemies[j];
				var sdx = e.x - other.x;
				var sdy = e.y - other.y;
				var sdist = Math.sqrt(sdx * sdx + sdy * sdy);
				var minSep = ENEMY_SIZE + 8.0;
				if (sdist > 0.001 && sdist < minSep) {
					var push = (minSep - sdist) / minSep;
					vx += (sdx / sdist) * e.speed * 0.65 * push;
					vy += (sdy / sdist) * e.speed * 0.65 * push;
				}
			}

			e.x += vx * dt;
			e.y += vy * dt;

			if (e.x < eMinX) e.x = eMinX;
			if (e.x > eMaxX) e.x = eMaxX;
			if (e.y < eMinY) e.y = eMinY;
			if (e.y > eMaxY) e.y = eMaxY;

			HashLoader.setField(e.bmp, "x", e.x);
			HashLoader.setField(e.bmp, "y", e.y);
			try {
				HashLoader.setField(e.bmp, "posChanged", true);
			} catch (_:Dynamic) {}

			if (Math.abs(px - e.x) < HIT_DIST_X && Math.abs(py - e.y) < HIT_DIST_Y) {
				return true;
			}
		}

		return false;
	}

	public static function reset() {
		for (e in enemies) {
			e.x = e.spawnX;
			e.y = e.spawnY;
			HashLoader.setField(e.bmp, "x", e.x);
			HashLoader.setField(e.bmp, "y", e.y);
			try {
				HashLoader.setField(e.bmp, "posChanged", true);
			} catch (_:Dynamic) {}
		}
	}
}

