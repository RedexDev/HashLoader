# ExampleMod (HaxeFlixel — FlxBunnyMark)

Mod de ejemplo para **FlxBunnyMark** (`hlboot.dat` + `FlxBunnyMark.exe`) usando el SDK de **HashLoader** (`hashloader.HashLoader`), dividido en módulos dentro de `src/`:

- `src/ModMain.hx` — Punto de entrada del mod, conexión con `flixel.FlxG` y `PlayState`, y bucle por frame.
- `src/HudBanner.hx` — Barra inferior HUD (`FlxSprite` + `FlxText`) integrada en `PlayState` con estado en vivo de los efectos.
- `src/BunnyController.hx` — Efecto arcoíris dinámico sobre los `Bunny`, selector de gravedad (Normal / Invertida / Cero), repulsor/atractor magnético con el ratón e impulso masivo.
- `src/InputHelper.hx` — Lectura segura de teclado (`FlxG.keys`) y ratón (`FlxG.mouse`) mediante reflexión.

## Controles dentro del juego

- **R**: Activar / desactivar modo Arcoíris en todos los conejos.
- **G**: Alternar gravedad (`Normal` → `Invertida` → `Gravedad Cero`).
- **C**: Impulso / explosión radial en todos los conejos.
- **Click Izquierdo (mantener)**: Repeler conejos alrededor del cursor.
- **Click Derecho (mantener)**: Atraer conejos hacia el cursor (vórtice).

## Compilar y empaquetar

1. Compila el bytecode del mod:
   ```powershell
   haxe build.hxml
   ```
2. Empaqueta la carpeta `ExampleMod(HaxeFlixel)` en un archivo `.hlmod` desde **HashToolkit** (*Empaquetar carpeta → .hlmod*) y colócalo en la carpeta `hlmods/` del juego.

