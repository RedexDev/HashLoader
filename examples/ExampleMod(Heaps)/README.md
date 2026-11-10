# Terex Bounds Mod

Mod de prueba para **HashLoader** sobre el juego de ejemplo `Terex-master`.

Hace dos cosas:

1. Dibuja un marco magenta visible en los bordes del area de juego y encierra
   al jugador dentro de el (no puede salirse).
2. Cambia el titulo de la ventana a `hash loader test!`.

## Instalacion

El paquete ya esta instalado en:

```
Terex-master/build/hlmods/terex_bounds_mod.hlmod
```

Para ejecutarlo basta con lanzar `Terex-master/build/HashLoader.exe`.

El loader extrae el paquete y carga `code/main.hl` como plugin. El puente
nativo (`hashloader.hdll`) ya viene embebido en `HashLoader.exe`; el runner lo
despliega en `<bin>/.hashloader/` y anade esa carpeta al `PATH`. Este mod **no**
lleva `.hdll`, `override/` ni codigo C propio.

## Que contiene el paquete

```
hlmod.json                 manifiesto del mod
code/main.hl               plugin compilado con Haxe/HashLink
```

## Como funciona

- El payload de HashLoader localiza el modulo `main.hl` del juego, engancha
  `hxd.System` y carga los mods en el hilo principal.
- El mod usa la API de runtime `hashloader.HashLoader` para resolver las
  clases del juego y acceder a su contexto:

  ```
  HashLoader.gameInstance()  -> instancia de Main (hxd.App)
  Main.player / Main.camera / Main.s2d
  ```

- **Marco:** cuatro `h2d.Bitmap` con un `h2d.Tile.fromColor` magenta,
  anadidos como hijos de la camara del juego.
- **Recorte:** cada frame se fijan `player.x` / `player.y` dentro del rango
  permitido y se marca `posChanged` (los setters y `moveAbsolute` del juego
  son `inline`, por eso se escribe el campo directamente).
- **Titulo:** `HashLoader.setWindowTitle("hash loader test!")`.
- **Callbacks:** `HashLoader.onBeforeFrame` / `HashLoader.onFrame`, ejecutados
  en el hilo principal del juego.

## Compilar

Requisitos: Haxe 4.x/5.x con HashLink JIT. No hace falta MSVC ni las cabeceras
de HashLink para este mod.

```powershell
# plugin Haxe -> code/main.hl
haxe build.hxml

# empaquetar -> Terex-master/build/hlmods/terex_bounds_mod.hlmod
powershell -ExecutionPolicy Bypass -File pack.ps1
```

El SDK Haxe se toma de `HashLoader-Src/app/sdk` mediante el `-cp` de
`build.hxml` (o de `tools/sdk` que instala el kit). El mod no lleva una copia
local de `hashloader/HashLoader.hx`.

`pack.ps1` empaqueta solo `hlmod.json`, `code/`, `override/` y assets; el
codigo fuente y los objetos de compilacion no viajan en el `.hlmod`.

## Ajustes

Los limites y el color estan en `src/ModMain.hx`:

- margen del area: `8` px en `setup()` / `createVisuals()`;
- color del marco: `0xFF00E5` en `createVisuals()`;
- titulo: constante en `setWindowTitle(...)`.

El mod depende de los nombres `Main`, `Player`, `Camera`, `h2d.*` y
`hxd.System` del juego Terex. Para otro juego habria que adaptar esos nombres,
pero no el codigo nativo: el puente de HashLoader es generico.

## Diagnostico

El mod escribe `terex_bounds_mod_log.txt` junto al ejecutable del juego con el
resultado de la inicializacion, el enganche al bucle y los recortes aplicados.
