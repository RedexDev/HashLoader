package;

import hashloader.HashLoader;

/**
 * Barra HUD inferior en `PlayState` que muestra el estado activo del mod y sus atajos.
 */
class HudBanner {
	var bgSprite:Dynamic = null;
	var labelText:Dynamic = null;
	var cursorRing:Dynamic = null;
	var lastText:String = "";

	public function new(state:Dynamic, flxG:Dynamic) {
		var spriteCls = HashLoader.resolveClass("flixel.FlxSprite");
		var textCls = HashLoader.resolveClass("flixel.text.FlxText");

		var gameW:Int = 640;
		var gameH:Int = 480;
		if (flxG != null) {
			var w:Dynamic = HashLoader.staticField(flxG, "width");
			var h:Dynamic = HashLoader.staticField(flxG, "height");
			if (w != null) gameW = (w : Int);
			if (h != null) gameH = (h : Int);
		}

		// Indicador visual del vórtice/repulsor del ratón
		if (spriteCls != null) {
			cursorRing = HashLoader.newInstance(spriteCls, [0.0, 0.0, null]);
			if (cursorRing != null) {
				HashLoader.call(cursorRing, "makeGraphic", [24, 24, 0x8800E5FF, false, "hlmod_cursor_ring"]);
				HashLoader.call(cursorRing, "set_visible", [false]);
				HashLoader.call(state, "add", [cursorRing]);
			}

			// Fondo semitransparente inferior
			var bgY:Float = gameH - 38;
			bgSprite = HashLoader.newInstance(spriteCls, [0.0, bgY, null]);
			if (bgSprite != null) {
				HashLoader.call(bgSprite, "makeGraphic", [gameW, 38, 0xDD10141F, false, "hlmod_hud_bg"]);
				HashLoader.call(bgSprite, "set_alpha", [0.88]);
				HashLoader.call(state, "add", [bgSprite]);
			}
		}

		// Texto informativo de controles y estado (con texto inicial desde el primer frame)
		if (textCls != null) {
			var initialTxt = buildStatusText(true, 0);
			lastText = initialTxt;
			var txtY:Float = gameH - 35;
			var txtW:Float = gameW - 12;
			labelText = HashLoader.newInstance(textCls, [6.0, txtY, txtW, initialTxt, 11, true]);
			if (labelText != null) {
				HashLoader.call(labelText, "set_color", [0xFF76FF03]);
				HashLoader.call(labelText, "set_alignment", ["center"]);
				HashLoader.call(state, "add", [labelText]);
			}
		}
	}

	static function buildStatusText(rainbowOn:Bool, gravityMode:Int):String {
		var gravName = switch (gravityMode) {
			case 1: "INVERTIDA";
			case 2: "CERO-G";
			default: "NORMAL";
		};
		var rState = rainbowOn ? "ON" : "OFF";
		return '[HashLoader Mod]  [R] Arcoiris: $rState  |  [G] Gravedad: $gravName  |  [C] Impulso\n'
			+ 'Click Izq: Repeler conejos  |  Click Der: Vortice atractor';
	}

	public function update(rainbowOn:Bool, gravityMode:Int, mouseMode:Int, mx:Float, my:Float) {
		if (cursorRing != null) {
			if (mouseMode != 0) {
				HashLoader.call(cursorRing, "set_visible", [true]);
				HashLoader.call(cursorRing, "set_x", [mx - 12.0]);
				HashLoader.call(cursorRing, "set_y", [my - 12.0]);
				var ringColor:Int = (mouseMode == 1) ? 0xFFFF5252 : 0xFF00E5FF;
				HashLoader.call(cursorRing, "set_color", [ringColor]);
			} else {
				HashLoader.call(cursorRing, "set_visible", [false]);
			}
		}

		if (labelText == null) return;

		var txt = buildStatusText(rainbowOn, gravityMode);
		if (txt != lastText) {
			lastText = txt;
			HashLoader.call(labelText, "set_text", [txt]);
		}
	}
}
