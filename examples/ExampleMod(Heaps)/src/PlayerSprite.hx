package;

import hashloader.HashLoader;
import sys.FileSystem;
import sys.io.File;

/**
 * Carga y aplica el sprite estático pixel-art del jugador desde `res/player.png`
 * (empaquetado dentro del propio mod en `examples/ExampleMod/res/player.png`).
 */
class PlayerSprite {
	// En format.hl.Native.PixelFormat (fmt.hdll), BGRA es 8 (1 era BGR de 24 bits sin canal alfa)
	static inline var FMT_BGRA:Int = 8;
	// Escala para que el pixel-art (22x61) se vea con buen tamaño y nítido en pantalla
	static inline var PLAYER_SCALE:Float = 1.5;

	public static var tile:Dynamic = null;

	@:hlNative("fmt", "png_decode")
	static function pngDecode(src:hl.Bytes, srcLen:Int, dst:hl.Bytes, width:Int, height:Int, stride:Int, format:Int, flags:Int):Bool {
		return false;
	}

	static function findSpritePath():String {
		var candidates:Array<String> = [];

		// 1. Buscar primero dentro de la carpeta extraída del propio mod (.hashloader/mods/*/res/player.png)
		try {
			if (FileSystem.exists(".hashloader/mods") && FileSystem.isDirectory(".hashloader/mods")) {
				for (entry in FileSystem.readDirectory(".hashloader/mods")) {
					var modAsset = ".hashloader/mods/" + entry + "/res/player.png";
					candidates.push(modAsset);
				}
			}
		} catch (_:Dynamic) {}

		// 2. Rutas relativas al directorio del juego / proyecto
		candidates.push("res/player.png");
		candidates.push("../res/player.png");
		candidates.push("../../res/player.png");
		candidates.push("../../../Terex-master/res/player.png");

		var rootEnv = Sys.getEnv("HASHLOADER_ROOT");
		if (rootEnv != null && rootEnv != "") {
			candidates.push(rootEnv + "/res/player.png");
			candidates.push(rootEnv + "/../res/player.png");
		}

		for (p in candidates) {
			if (FileSystem.exists(p) && !FileSystem.isDirectory(p)) {
				return p;
			}
		}
		return null;
	}

	public static function load(player:Dynamic) {
		var path = findSpritePath();
		if (path == null) {
			ModMain.log("AVISO: no se encontro res/player.png");
			return;
		}

		var rawBytes = File.getBytes(path);
		if (rawBytes.length < 24) {
			ModMain.log("AVISO: archivo player.png invalido o demasiado pequeno");
			return;
		}

		// Leer dimensiones reales desde la cabecera IHDR del PNG (bytes 16..23 big-endian)
		var imgW = (rawBytes.get(16) << 24) | (rawBytes.get(17) << 16) | (rawBytes.get(18) << 8) | rawBytes.get(19);
		var imgH = (rawBytes.get(20) << 24) | (rawBytes.get(21) << 16) | (rawBytes.get(22) << 8) | rawBytes.get(23);
		if (imgW <= 0 || imgH <= 0) {
			imgW = 22;
			imgH = 61;
		}

		var rawPtr:hl.Bytes = hl.Bytes.fromBytes(rawBytes);

		tile = loadTileViaFmtAndBitmapData(rawPtr, rawBytes.length, imgW, imgH);
		if (tile == null) {
			tile = loadTileViaAnyResource(rawPtr, rawBytes.length);
		}

		if (tile != null) {
			applyToPlayer(player);
			MapBounds.updatePlayerBounds(player);
			ModMain.log("sprite pixel-art del jugador cargado desde '" + path + "' (" + imgW + "x" + imgH + ", escala=" + PLAYER_SCALE + ")");
		} else {
			ModMain.log("AVISO: no se pudo construir el Tile para player.png");
		}
	}

	public static function applyToPlayer(player:Dynamic) {
		if (tile == null || player == null) return;
		HashLoader.setField(player, "tile", tile);
		HashLoader.setField(player, "scaleX", PLAYER_SCALE);
		HashLoader.setField(player, "scaleY", PLAYER_SCALE);
		// Desactivar suavizado bilinear para que el pixel-art se vea nítido (Nearest)
		try {
			HashLoader.setField(player, "smooth", false);
		} catch (_:Dynamic) {}
		try {
			HashLoader.setField(player, "posChanged", true);
		} catch (_:Dynamic) {}
	}

	/**
	 * Decodifica el PNG con `fmt.png_decode` en formato BGRA (`FMT_BGRA = 8`)
	 * y construye el `h2d.Tile` mediante `hxd.BitmapData` del juego.
	 */
	static function loadTileViaFmtAndBitmapData(rawPtr:hl.Bytes, rawLen:Int, imgW:Int, imgH:Int):Dynamic {
		try {
			var bmdCls:Dynamic = HashLoader.resolveClass("hxd.BitmapData");
			var tileCls:Dynamic = HashLoader.resolveClass("h2d.Tile");
			if (bmdCls == null || tileCls == null) return null;

			var byteSize = imgW * imgH * 4;
			var decoded = new hl.Bytes(byteSize);
			decoded.fill(0, byteSize, 0);

			// FMT_BGRA = 8 en format.hl.Native.PixelFormat (32 bits BGRA con canal alfa)
			if (!pngDecode(rawPtr, rawLen, decoded, imgW, imgH, imgW * 4, FMT_BGRA, 0)) {
				return null;
			}

			var bmd:Dynamic = HashLoader.newInstance(bmdCls, [imgW, imgH]);
			if (bmd == null) return null;

			for (py in 0...imgH) {
				var px = 0;
				while (px < imgW) {
					var offset = ((py * imgW) + px) << 2;
					var color = decoded.getI32(offset);
					var alpha = (color >>> 24) & 0xFF;
					if (alpha == 0) {
						px++;
						continue;
					}
					var run = 1;
					while (px + run < imgW) {
						var nextOff = ((py * imgW) + (px + run)) << 2;
						if (decoded.getI32(nextOff) != color) break;
						run++;
					}
					HashLoader.call(bmd, "fill", [px, py, run, 1, color]);
					px += run;
				}
			}

			var rawTile:Dynamic = HashLoader.callStatic(tileCls, "fromBitmap", [bmd]);
			HashLoader.call(bmd, "dispose", []);
			if (rawTile == null) return null;

			var centered:Dynamic = HashLoader.call(rawTile, "center", []);
			return centered != null ? centered : rawTile;
		} catch (e:Dynamic) {
			ModMain.log("aviso en loadTileViaFmtAndBitmapData: " + Std.string(e));
			return null;
		}
	}

	/**
	 * Método de respaldo usando `hxd.res.Any.fromBytes` con `haxe.io.Bytes` del juego.
	 */
	static function loadTileViaAnyResource(rawPtr:hl.Bytes, rawLen:Int):Dynamic {
		try {
			var bytesCls:Dynamic = HashLoader.resolveClass("haxe.io.Bytes");
			var anyCls:Dynamic = HashLoader.resolveClass("hxd.res.Any");
			if (bytesCls == null || anyCls == null) return null;

			var gameBytes:Dynamic = HashLoader.newInstance(bytesCls, [rawPtr, rawLen]);
			if (gameBytes == null) return null;

			var anyRes:Dynamic = HashLoader.callStatic(anyCls, "fromBytes", ["player.png", gameBytes]);
			if (anyRes == null) return null;

			var rawTile:Dynamic = HashLoader.call(anyRes, "toTile", []);
			if (rawTile == null) return null;

			var centered:Dynamic = HashLoader.call(rawTile, "center", []);
			return centered != null ? centered : rawTile;
		} catch (e:Dynamic) {
			ModMain.log("aviso en loadTileViaAnyResource: " + Std.string(e));
			return null;
		}
	}
}
