package;

import hashloader.HashLoader;

/**
 * Lectura segura y rápida de entradas de teclado (`FlxG.keys`) y ratón (`FlxG.mouse`)
 * para juegos HaxeFlixel cargados por reflexión, evitando `cast` cruzados de `ArrayObj`
 * entre módulos y cacheando las referencias a cada tecla en O(1).
 */
class InputHelper {
	// Códigos de tecla (ASCII mayúscula en FlxKey)
	public static inline var KEY_R:Int = 82;
	public static inline var KEY_G:Int = 71;
	public static inline var KEY_C:Int = 67;

	// Estados de FlxInputState
	static inline var STATE_PRESSED:Int = 1;
	static inline var STATE_JUST_PRESSED:Int = 2;

	static var cachedKeysObj:Dynamic = null;
	static var keyMap:Map<Int, Dynamic> = new Map();

	/** Devuelve `true` solo en el frame en que se pulsa la tecla `keyCode`. */
	public static function isKeyJustPressed(flxG:Dynamic, keyCode:Int):Bool {
		var k = getKeyInput(flxG, keyCode);
		if (k == null) return false;
		var cur:Dynamic = HashLoader.field(k, "current");
		return cur != null && (cur : Int) == STATE_JUST_PRESSED;
	}

	/** Devuelve `true` mientras la tecla `keyCode` se mantiene pulsada. */
	public static function isKeyPressed(flxG:Dynamic, keyCode:Int):Bool {
		var k = getKeyInput(flxG, keyCode);
		if (k == null) return false;
		var cur:Dynamic = HashLoader.field(k, "current");
		if (cur == null) return false;
		var st:Int = (cur : Int);
		return st == STATE_PRESSED || st == STATE_JUST_PRESSED;
	}

	static function getKeyInput(flxG:Dynamic, keyCode:Int):Dynamic {
		if (flxG == null) return null;
		var keys:Dynamic = HashLoader.staticField(flxG, "keys");
		if (keys == null) return null;

		if (keys != cachedKeysObj) {
			cachedKeysObj = keys;
			keyMap = new Map();
		}

		if (keyMap.exists(keyCode)) {
			return keyMap.get(keyCode);
		}

		var rawList:Dynamic = HashLoader.field(keys, "_keyListArray");
		if (rawList == null) return null;

		var len = HashLoader.arrayLength(rawList);
		for (i in 0...len) {
			var item:Dynamic = HashLoader.arrayGet(rawList, i);
			if (item != null) {
				var id:Dynamic = HashLoader.field(item, "ID");
				if (id != null && (id : Int) == keyCode) {
					keyMap.set(keyCode, item);
					return item;
				}
			}
		}
		return null;
	}

	/** Posición X del cursor en coordenadas del juego (`FlxG.mouse.x`). */
	public static function mouseX(flxG:Dynamic):Float {
		var mouse:Dynamic = HashLoader.staticField(flxG, "mouse");
		if (mouse == null) return 0.0;
		var x:Dynamic = HashLoader.field(mouse, "x");
		return x != null ? (x : Float) : 0.0;
	}

	/** Posición Y del cursor en coordenadas del juego (`FlxG.mouse.y`). */
	public static function mouseY(flxG:Dynamic):Float {
		var mouse:Dynamic = HashLoader.staticField(flxG, "mouse");
		if (mouse == null) return 0.0;
		var y:Dynamic = HashLoader.field(mouse, "y");
		return y != null ? (y : Float) : 0.0;
	}

	/** `true` si el botón izquierdo del ratón está presionado (fuera de la barra superior de UI). */
	public static function isMouseLeftDown(flxG:Dynamic):Bool {
		var mouse:Dynamic = HashLoader.staticField(flxG, "mouse");
		if (mouse == null) return false;
		if (mouseY(flxG) < 100.0) return false; // no interferir con los botones superiores de PlayState
		var btn:Dynamic = HashLoader.field(mouse, "_leftButton");
		if (btn == null) return false;
		var cur:Dynamic = HashLoader.field(btn, "current");
		if (cur == null) return false;
		var st:Int = (cur : Int);
		return st == STATE_PRESSED || st == STATE_JUST_PRESSED;
	}

	/** `true` si el botón derecho del ratón está presionado. */
	public static function isMouseRightDown(flxG:Dynamic):Bool {
		var mouse:Dynamic = HashLoader.staticField(flxG, "mouse");
		if (mouse == null) return false;
		var btn:Dynamic = HashLoader.field(mouse, "_rightButton");
		if (btn == null) return false;
		var cur:Dynamic = HashLoader.field(btn, "current");
		if (cur == null) return false;
		var st:Int = (cur : Int);
		return st == STATE_PRESSED || st == STATE_JUST_PRESSED;
	}
}
