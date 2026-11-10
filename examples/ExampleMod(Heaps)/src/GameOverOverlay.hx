package;

import hashloader.HashLoader;

/**
 * Controla el estado de Game Over, la cuenta regresiva de 3 segundos,
 * el cartel pixel-art en pantalla y el reinicio de la partida.
 */
class GameOverOverlay {
	public static inline var DURATION:Float = 3.0;

	public static var active:Bool = false;
	static var timer:Float = 0.0;
	static var lastCountdownSec:Int = -1;
	static var deathX:Float = 250.0;
	static var deathY:Float = 250.0;

	static var bannerBmp:Dynamic = null;
	static var bannerTiles:Array<Dynamic> = [];

	public static function createOverlay(s2d:Dynamic) {
		var tileCls:Dynamic = HashLoader.resolveClass("h2d.Tile");
		var bmpCls:Dynamic = HashLoader.resolveClass("h2d.Bitmap");
		var bmdCls:Dynamic = HashLoader.resolveClass("hxd.BitmapData");
		if (tileCls == null || bmpCls == null || bmdCls == null) return;

		var bannerW = 360;
		var bannerH = 120;
		bannerTiles = [];

		for (sec in 1...4) {
			var bmd:Dynamic = HashLoader.newInstance(bmdCls, [bannerW, bannerH]);
			// Marco exterior rojo intenso + fondo oscuro
			HashLoader.call(bmd, "fill", [0, 0, bannerW, bannerH, 0xFFFF1744]);
			HashLoader.call(bmd, "fill", [4, 4, bannerW - 8, bannerH - 8, 0xFF140A1E]);

			// Texto grande "GAME OVER" (escala x5)
			drawPixelText(bmd, "GAME OVER", 48, 22, 5, 0xFFFF5252);
			// Subtítulo "REINICIO EN X" (escala x3)
			drawPixelText(bmd, "REINICIO EN " + sec, 66, 76, 3, 0xFFFFEA00);

			var tile:Dynamic = HashLoader.callStatic(tileCls, "fromBitmap", [bmd]);
			HashLoader.call(bmd, "dispose", []);
			if (tile != null) {
				var centered:Dynamic = HashLoader.call(tile, "center", []);
				bannerTiles.push(centered != null ? centered : tile);
			}
		}

		if (bannerTiles.length > 0) {
			bannerBmp = HashLoader.newInstance(bmpCls, [bannerTiles[2], s2d]);
			HashLoader.setField(bannerBmp, "x", MapBounds.worldW * 0.5);
			HashLoader.setField(bannerBmp, "y", MapBounds.worldH * 0.5);
			HashLoader.setField(bannerBmp, "visible", false);
		}
	}

	public static function trigger(player:Dynamic, camera:Dynamic) {
		if (active) return;
		active = true;
		timer = DURATION;
		lastCountdownSec = -1;

		deathX = HashLoader.field(player, "x");
		deathY = HashLoader.field(player, "y");
		freezePlayer(player, camera);

		if (bannerBmp != null) {
			HashLoader.setField(bannerBmp, "visible", true);
		}
		updateBanner(3);
		ModMain.log("GAME OVER: jugador alcanzado en (" + Std.int(deathX) + "," + Std.int(deathY) + "). Reiniciando en 3s...");
	}

	public static function freezePlayer(player:Dynamic, camera:Dynamic) {
		HashLoader.setField(player, "x", deathX);
		HashLoader.setField(player, "y", deathY);
		HashLoader.setField(player, "xMove", 0.0);
		HashLoader.setField(player, "yMove", 0.0);
		try {
			HashLoader.setField(player, "posChanged", true);
			if (camera != null) {
				HashLoader.call(camera, "set_viewX", [deathX]);
				HashLoader.call(camera, "set_viewY", [deathY]);
			}
		} catch (_:Dynamic) {}
	}

	public static function update(player:Dynamic, camera:Dynamic, spawnX:Float, spawnY:Float, dt:Float) {
		if (!active) return;

		timer -= dt;
		var secLeft = Math.ceil(timer);
		if (secLeft < 1) secLeft = 1;
		if (secLeft > 3) secLeft = 3;

		if (secLeft != lastCountdownSec) {
			lastCountdownSec = secLeft;
			updateBanner(secLeft);
		}

		if (timer <= 0.0) {
			resetRound(player, camera, spawnX, spawnY);
		}
	}

	static function updateBanner(secLeft:Int) {
		var idx = secLeft - 1;
		if (bannerBmp != null && idx >= 0 && idx < bannerTiles.length && bannerTiles[idx] != null) {
			HashLoader.setField(bannerBmp, "tile", bannerTiles[idx]);
		}
		HashLoader.setWindowTitle("GAME OVER! Reiniciando en " + secLeft + "s...");
	}

	static function resetRound(player:Dynamic, camera:Dynamic, spawnX:Float, spawnY:Float) {
		active = false;
		timer = 0.0;
		lastCountdownSec = -1;

		if (bannerBmp != null) {
			HashLoader.setField(bannerBmp, "visible", false);
		}

		// Restaurar jugador en su posición inicial y asegurar su sprite
		HashLoader.setField(player, "x", spawnX);
		HashLoader.setField(player, "y", spawnY);
		HashLoader.setField(player, "xMove", 0.0);
		HashLoader.setField(player, "yMove", 0.0);
		PlayerSprite.applyToPlayer(player);

		try {
			HashLoader.setField(player, "posChanged", true);
			if (camera != null) {
				HashLoader.call(camera, "set_viewX", [spawnX]);
				HashLoader.call(camera, "set_viewY", [spawnY]);
			}
		} catch (_:Dynamic) {}

		// Restaurar los 3 cubos azules a sus posiciones iniciales
		EnemyManager.reset();

		HashLoader.setWindowTitle("hash loader test!");
		ModMain.log("Partida reiniciada");
	}

	static function drawPixelText(bmd:Dynamic, text:String, startX:Int, startY:Int, scale:Int, color:Int) {
		var cx = startX;
		for (i in 0...text.length) {
			var ch = text.charAt(i);
			var rows = glyphRows(ch);
			for (ry in 0...rows.length) {
				var row = rows[ry];
				for (rx in 0...5) {
					if (((row >> (4 - rx)) & 1) != 0) {
						HashLoader.call(bmd, "fill", [cx + rx * scale, startY + ry * scale, scale, scale, color]);
					}
				}
			}
			cx += 6 * scale;
		}
	}

	static function glyphRows(ch:String):Array<Int> {
		return switch (ch) {
			case "G": [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0E];
			case "A": [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11];
			case "M": [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11];
			case "E": [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F];
			case "O": [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E];
			case "V": [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04];
			case "R": [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11];
			case "I": [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E];
			case "N": [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11];
			case "C": [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E];
			case "1": [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E];
			case "2": [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F];
			case "3": [0x1E, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x1E];
			default:  [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
		};
	}
}

