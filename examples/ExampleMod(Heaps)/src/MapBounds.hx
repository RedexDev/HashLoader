package;

import hashloader.HashLoader;

/**
 * Gestiona los límites del mapa: cálculo del área jugable, dibujo del marco
 * magenta en la cámara y restricción (clamp) de la posición del jugador.
 */
class MapBounds {
	public static var worldW:Float = 800;
	public static var worldH:Float = 600;
	public static var minX:Float = 8;
	public static var minY:Float = 8;
	public static var maxX:Float = 792;
	public static var maxY:Float = 592;
	public static var clampedCount:Int = 0;

	public static function init(s2d:Dynamic, player:Dynamic) {
		worldW = HashLoader.field(s2d, "width");
		worldH = HashLoader.field(s2d, "height");
		updatePlayerBounds(player);
	}

	public static function updatePlayerBounds(player:Dynamic) {
		minX = 8;
		minY = 8;
		maxX = Std.int(worldW) - 8;
		maxY = Std.int(worldH) - 8;
		try {
			var tile:Dynamic = HashLoader.field(player, "tile");
			var tw:Float = HashLoader.field(tile, "width");
			var th:Float = HashLoader.field(tile, "height");
			var sx:Float = HashLoader.field(player, "scaleX");
			var sy:Float = HashLoader.field(player, "scaleY");
			if (sx == 0) sx = 1;
			if (sy == 0) sy = 1;
			minX += tw * sx * 0.5;
			minY += th * sy * 0.5;
			maxX -= tw * sx * 0.5;
			maxY -= th * sy * 0.5;
			ModMain.log("sprite " + tw + "x" + th + " escala " + sx + "x" + sy
				+ " -> rango jugador [" + minX + "," + minY + "] - [" + maxX + "," + maxY + "]");
		} catch (_:Dynamic) {}
	}

	public static function clampPlayer(player:Dynamic) {
		var x:Float = HashLoader.field(player, "x");
		var y:Float = HashLoader.field(player, "y");
		var nx:Float = x;
		var ny:Float = y;

		if (nx < minX) nx = minX;
		if (nx > maxX) nx = maxX;
		if (ny < minY) ny = minY;
		if (ny > maxY) ny = maxY;

		if (nx != x || ny != y) {
			HashLoader.setField(player, "x", nx);
			HashLoader.setField(player, "y", ny);
			try {
				HashLoader.setField(player, "posChanged", true);
			} catch (_:Dynamic) {}
			clampedCount++;
		}
	}

	public static function createVisuals(camera:Dynamic) {
		var tileCls:Dynamic = HashLoader.resolveClass("h2d.Tile");
		var bmpCls:Dynamic = HashLoader.resolveClass("h2d.Bitmap");
		if (tileCls == null || bmpCls == null) {
			ModMain.log("ERROR: faltan clases para dibujar (h2d.Tile=" + (tileCls != null)
				+ ", h2d.Bitmap=" + (bmpCls != null) + ")");
			return;
		}

		var fromColor:Dynamic = HashLoader.staticField(tileCls, "fromColor");
		if (fromColor == null) throw "h2d.Tile.fromColor no disponible";

		// Un único tile magenta reutilizado por las cuatro franjas.
		var tile:Dynamic = Reflect.callMethod(tileCls, fromColor, [0xFF00E5, 8, 8]);
		var tsize = 8.0;

		var bx0 = 8.0;
		var by0 = 8.0;
		var bx1 = worldW - 8;
		var by1 = worldH - 8;

		addStrip(bmpCls, tile, camera, bx0, by0, bx1 - bx0, 6, tsize); // arriba
		addStrip(bmpCls, tile, camera, bx0, by1 - 6, bx1 - bx0, 6, tsize); // abajo
		addStrip(bmpCls, tile, camera, bx0, by0, 6, by1 - by0, tsize); // izquierda
		addStrip(bmpCls, tile, camera, bx1 - 6, by0, 6, by1 - by0, tsize); // derecha

		// Esquinas más gruesas para que se vean bien.
		var c = 24.0;
		addStrip(bmpCls, tile, camera, bx0, by0, c, 14, tsize);
		addStrip(bmpCls, tile, camera, bx1 - c, by0, c, 14, tsize);
		addStrip(bmpCls, tile, camera, bx0, by1 - 14, c, 14, tsize);
		addStrip(bmpCls, tile, camera, bx1 - c, by1 - 14, c, 14, tsize);

		ModMain.log("limites dibujados: " + bx0 + "," + by0 + " - " + bx1 + "," + by1
			+ " (jugador hasta " + maxX + "," + maxY + ")");
	}

	static function addStrip(bmpCls:Dynamic, tile:Dynamic, parent:Dynamic,
			x:Float, y:Float, w:Float, h:Float, tsize:Float):Dynamic {
		var b:Dynamic = HashLoader.newInstance(bmpCls, [tile, parent]);
		if (b == null) throw "no se pudo crear h2d.Bitmap";
		HashLoader.setField(b, "x", x);
		HashLoader.setField(b, "y", y);
		HashLoader.setField(b, "scaleX", w / tsize);
		HashLoader.setField(b, "scaleY", h / tsize);
		return b;
	}
}

