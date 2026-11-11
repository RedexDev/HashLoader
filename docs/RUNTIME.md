# Runner and Runtime Reference

## Runner execution contract

The runner writes `<bin>/.hashloader/loader.json` and invokes the HashLink JIT host:

```text
hashlink.exe [payload.hl] <main.hl> [game arguments]
  cwd               = <bin>
  PATH              = <bin>;<hashlink folder>;<libhl folder>;<bin>/.hashloader
  HASHLOADER_CONFIG = <bin>/.hashloader/loader.json
  HASHLOADER_ROOT   = <root>
  Logs              = <bin>/.hashloader/run.log, run.stdout, run.stderr
```

The reference payload is `app/LoaderPayload.hx`, compiled to `app/HashLoader.hl`. It runs as the primary entry-point bootstrap and dynamically links the game as a plugin:

```haxe
class LoaderPayload {
  static function main() {
    final cfg = Sys.getEnv("HASHLOADER_CONFIG"); // loader.json
    // 1) Parse loader.json (mods sorted by priority)
    // 2) Initialize native bridge observer: locate hxd.System,
    //    openfl/lime, or engine loop and hook its tick into the main thread
    // 3) Load the game via sys_load_plugin(main_hl)
  }
}
```

`loader.json` is regenerated on each launch. Its structure is:

```json
{
  "version": 1,
  "bin": "PATH\\bin",
  "main_hl": "PATH\\bin\\main.hl",
  "payload": "PATH\\bin\\.hashloader\\HashLoader.hl",
  "mods": [
    {
      "id": "my-mod",
      "name": "My Mod",
      "version": "1.0.0",
      "author": "your-username",
      "path": "PATH\\bin\\.hashloader\\mods\\my-mod-1.0.0\\code\\main.hl",
      "priority": 100
    }
  ]
}
```
