package;

import hashloader.HashLoader;

/**
 * Punto de entrada del mod de ejemplo para FlxBunnyMark (HaxeFlixel + hlboot.dat).
 *
 * Estructura modular en `src/`:
 * - `ModMain.hx`         : Inicialización, detección de `PlayState` y bucle por frame.
 * - `HudBanner.hx`       : HUD inferior (`FlxSprite` + `FlxText`) y anillo indicador del cursor.
 * - `BunnyController.hx` : Efecto arcoíris, gravedad configurable, vórtice de ratón e impulso.
 * - `InputHelper.hx`     : Lectura de teclado (`FlxG.keys`) y ratón (`FlxG.mouse`).
 */
class ModMain {
	static var flxG:Dynamic = null;
	static var initializedState:Dynamic = null;
	static var hud:HudBanner = null;
	static var controller:BunnyController = null;

	public static function main() {
		Sys.println("[FlxBunnyMarkMod] Inicializando mod de HaxeFlixel...");

		if (!HashLoader.available()) {
			Sys.println("[FlxBunnyMarkMod] Puente nativo no disponible.");
			return;
		}

		HashLoader.setWindowTitle("FlxBunnyMark — HashLoader Mod!");
		controller = new BunnyController();

		HashLoader.onFrame(onFrameUpdate);
	}

	static function onFrameUpdate() {
		if (flxG == null) {
			flxG = HashLoader.flxG();
			if (flxG == null) return;
		}

		var game:Dynamic = HashLoader.staticField(flxG, "game");
		if (game == null) return;
		var state:Dynamic = HashLoader.field(game, "_state");
		if (state == null) return;

		// Esperar a que PlayState haya creado el grupo `_bunnies`
		if (HashLoader.field(state, "_bunnies") == null) return;

		// Si el estado acaba de crearse o reiniciarse, inyectamos el HUD
		if (initializedState != state) {
			initializedState = state;
			hud = new HudBanner(state, flxG);
			HashLoader.setWindowTitle("FlxBunnyMark — HashLoader Mod!");
			Sys.println("[FlxBunnyMarkMod] PlayState enganchado con exito.");
		}

		// Controles de teclado
		if (InputHelper.isKeyJustPressed(flxG, InputHelper.KEY_R)) {
			controller.toggleRainbow(state);
		}
		if (InputHelper.isKeyJustPressed(flxG, InputHelper.KEY_G)) {
			controller.cycleGravity(state);
		}
		if (InputHelper.isKeyJustPressed(flxG, InputHelper.KEY_C)) {
			controller.triggerBurst(state);
		}

		// Estado del ratón (0 = inactivo, 1 = repeler con click izq, 2 = atraer con click der)
		var mx = InputHelper.mouseX(flxG);
		var my = InputHelper.mouseY(flxG);
		var mouseMode:Int = 0;
		if (InputHelper.isMouseLeftDown(flxG)) {
			mouseMode = 1;
		} else if (InputHelper.isMouseRightDown(flxG)) {
			mouseMode = 2;
		}

		controller.update(state, mouseMode, mx, my);
		if (hud != null) {
			hud.update(controller.rainbowEnabled, controller.gravityMode, mouseMode, mx, my);
		}
	}
}
