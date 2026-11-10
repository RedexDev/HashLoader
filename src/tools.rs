use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum ToolsStatus {
    NotInstalled,
    Installing { progress: f32, msg: String },
    Installed,
}

pub fn tools_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("USERPROFILE")
                .map(|p| PathBuf::from(p).join("AppData").join("Local"))
                .unwrap_or_else(|_| std::env::temp_dir())
        });
    base.join("HashLoader").join("tools")
}

pub fn is_installed() -> bool {
    let marker = tools_dir().join(".installed");
    marker.is_file()
}

pub fn status(progress_state: &Option<(f32, String)>) -> ToolsStatus {
    if let Some((p, msg)) = progress_state {
        ToolsStatus::Installing {
            progress: *p,
            msg: msg.clone(),
        }
    } else if is_installed() {
        ToolsStatus::Installed
    } else {
        ToolsStatus::NotInstalled
    }
}

const HASH_HOOK_HX: &str = r#"package;

#if hl
import hl.types.ArrayObj;

/**
 * HashHook — API de Hooks dinámicos e interceptación para HashLoader.
 * Permite interceptar o reemplazar métodos en juegos HashLink en tiempo de ejecución
 * sin necesidad de poseer el código fuente del juego original.
 */
class HashHook {

	/**
	 * Resuelve una clase del juego anfitrión mediante su nombre calificado.
	 * Ejemplo: HashHook.resolveClass("PlayState")
	 */
	public static function resolveClass(className:String):Dynamic {
		try {
			var cls = Type.resolveClass(className);
			if (cls != null) return cls;
		} catch (_:Dynamic) {}

		// Búsqueda en el registro global de tipos de HashLink
		try {
			var map:hl.types.BytesMap = untyped $allTypes();
			if (map != null) {
				var b = @:privateAccess Sys.getPath(className);
				var t = map.get(b);
				if (t != null) return t;
			}
		} catch (_:Dynamic) {}
		return null;
	}

	/**
	 * Intercepta un método de una clase del juego anfitrión.
	 * `hookFn` se ejecutará con los argumentos que recibe el método.
	 */
	public static function hookMethod(targetClass:Dynamic, methodName:String, hookFn:Dynamic):Bool {
		if (targetClass == null || methodName == null || hookFn == null) return false;
		try {
			var originalFn = Reflect.field(targetClass, methodName);
			Reflect.setField(targetClass, methodName, Reflect.makeVarArgs(function(args:Array<Dynamic>) {
				try {
					hookFn(args);
				} catch (e:Dynamic) {
					Sys.println("[HashHook] Error en hook de " + methodName + ": " + Std.string(e));
				}
				if (originalFn != null) {
					return Reflect.callMethod(targetClass, originalFn, args);
				}
				return null;
			}));
			return true;
		} catch (e:Dynamic) {
			Sys.println("[HashHook] No se pudo enganchar " + methodName + ": " + Std.string(e));
			return false;
		}
	}

	/**
	 * Reemplaza completamente la implementación de un método en una clase.
	 */
	public static function replaceMethod(targetClass:Dynamic, methodName:String, newFn:Dynamic):Bool {
		if (targetClass == null || methodName == null || newFn == null) return false;
		try {
			Reflect.setField(targetClass, methodName, newFn);
			return true;
		} catch (e:Dynamic) {
			Sys.println("[HashHook] No se pudo reemplazar " + methodName + ": " + Std.string(e));
			return false;
		}
	}
}
#end
"#;

const TEMPLATE_BUILD_HXML: &str = r#"# Plantilla de compilación para Mods de HashLoader
-cp src
-cp stubs
-main Main
-hl code/main.hl
-dce std
--no-traces
"#;

const TEMPLATE_HLMOD_JSON: &str = r#"{
  "hlmod": 1,
  "id": "mi_primer_mod",
  "name": "Mi Primer Mod",
  "version": "1.0.0",
  "author": "Tu Nombre",
  "priority": 100,
  "main": "code/main.hl",
  "dependencies": []
}
"#;

const TEMPLATE_MAIN_HX: &str = r#"package;

import hashloader.HashLoader;

class Main {
	public static function main() {
		Sys.println("[MiMod] Mod cargado exitosamente via HashLoader!");

		// API de runtime: acceso directo a las clases del juego.
		if (!HashLoader.available()) {
			Sys.println("[MiMod] El puente de HashLoader no esta disponible.");
			return;
		}

		var main = HashLoader.gameInstance();
		var scene = HashLoader.scene();
		Sys.println("[MiMod] Main=" + (main != null) + " scene=" + (scene != null));

		// Cambiar el titulo de la ventana (Heaps/SDL).
		HashLoader.setWindowTitle("Mi Mod + HashLoader");

		// Logica por frame, ejecutada en el hilo principal del juego.
		var frames = 0;
		HashLoader.onFrame(function() {
			frames++;
			if (frames == 60) Sys.println("[MiMod] 60 frames ejecutados");
		});

		// Ejemplo de HashHook (hooks de metodos):
		// var playState = HashHook.resolveClass("PlayState");
	}
}
"#;

// Fuente del SDK de runtime: se incluye desde el propio árbol para que el
// instalador de herramientas y el repositorio nunca se desincronicen.
const HASHLOADER_SDK_HX: &str = include_str!("../app/sdk/hashloader/HashLoader.hx");

/// Ejecuta la instalación con pasos reales, progreso y reporte a los registros.
pub fn install_tools_background(
    log: Arc<Mutex<Vec<String>>>,
    progress_slot: Arc<Mutex<Option<(f32, String)>>>,
) {
    thread::spawn(move || {
        let log_msg = |msg: &str| {
            if let Ok(mut l) = log.lock() {
                l.push(msg.to_string());
            }
        };

        let update_prog = |p: f32, msg: &str| {
            if let Ok(mut slot) = progress_slot.lock() {
                *slot = Some((p, msg.to_string()));
            }
        };

        log_msg("[SDK] Iniciando instalación de Herramientas de Modding y Reverse Engineering...");
        update_prog(0.05, "Creando directorios locales de herramientas…");
        thread::sleep(Duration::from_millis(150));

        let tdir = tools_dir();
        let sdk_dir = tdir.join("sdk");
        let stubs_dir = tdir.join("stubs");
        let tpl_dir = tdir.join("templates");
        let tpl_src = tpl_dir.join("src");

        if let Err(e) = fs::create_dir_all(&sdk_dir).and_then(|_| fs::create_dir_all(&stubs_dir)).and_then(|_| fs::create_dir_all(&tpl_src)) {
            log_msg(&format!("[SDK ERROR] No se pudieron crear carpetas: {e}"));
            if let Ok(mut slot) = progress_slot.lock() {
                *slot = None;
            }
            return;
        }

        update_prog(0.25, "Instalando SDK de Hooks en tiempo de ejecución (HashHook.hx)…");
        thread::sleep(Duration::from_millis(200));
        let hook_file = sdk_dir.join("HashHook.hx");
        let _ = fs::write(&hook_file, HASH_HOOK_HX);
        log_msg(&format!("[SDK] Desplegado: {}", hook_file.display()));

        // API de runtime (hashloader.HashLoader) para mods de Heaps/HashLink.
        let bridge_sdk_dir = sdk_dir.join("hashloader");
        let _ = fs::create_dir_all(&bridge_sdk_dir);
        let bridge_sdk = bridge_sdk_dir.join("HashLoader.hx");
        let _ = fs::write(&bridge_sdk, HASHLOADER_SDK_HX);
        log_msg(&format!("[SDK] Desplegado: {}", bridge_sdk.display()));

        update_prog(0.50, "Instalando motor generador de Stubs y Externs Haxe…");
        thread::sleep(Duration::from_millis(200));
        let stubs_readme = stubs_dir.join("README.txt");
        let _ = fs::write(
            &stubs_readme,
            "Carpeta base de stubs de juegos analizados.\nUsa el boton 'Generar Stubs' en el Toolkit para analizar cualquier main.hl.\n",
        );
        log_msg("[SDK] Motor generador de stubs (.hx) registrado.");

        update_prog(0.75, "Configurando plantillas de desarrollo de mods (.hlmod)…");
        thread::sleep(Duration::from_millis(200));
        let _ = fs::write(tpl_dir.join("build.hxml"), TEMPLATE_BUILD_HXML);
        let _ = fs::write(tpl_dir.join("hlmod.json"), TEMPLATE_HLMOD_JSON);
        let _ = fs::write(tpl_src.join("Main.hx"), TEMPLATE_MAIN_HX);
        let _ = fs::copy(&hook_file, tpl_src.join("HashHook.hx"));
        let tpl_bridge_dir = tpl_src.join("hashloader");
        let _ = fs::create_dir_all(&tpl_bridge_dir);
        let _ = fs::write(tpl_bridge_dir.join("HashLoader.hx"), HASHLOADER_SDK_HX);
        log_msg("[SDK] Plantilla base de mod creada en tools/templates/.");

        update_prog(0.90, "Registrando herramientas gráficas (Descompilador e Inspector HL)…");
        thread::sleep(Duration::from_millis(200));
        let marker = tdir.join(".installed");
        let install_info = format!(
            "version={}\ninstalled_at={}\n",
            env!("CARGO_PKG_VERSION"),
            chrono_like_time()
        );
        let _ = fs::write(&marker, install_info);

        update_prog(1.0, "Instalación completada.");
        thread::sleep(Duration::from_millis(300));
        log_msg("[SDK] ¡Suite de herramientas de modding instalada con éxito!");

        if let Ok(mut slot) = progress_slot.lock() {
            *slot = None;
        }
    });
}

/// Desinstala las herramientas y limpia el entorno.
pub fn uninstall_tools(log: Arc<Mutex<Vec<String>>>) -> Result<(), String> {
    let tdir = tools_dir();
    if tdir.exists() {
        fs::remove_dir_all(&tdir).map_err(|e| format!("Error eliminando {}: {e}", tdir.display()))?;
    }
    if let Ok(mut l) = log.lock() {
        l.push("[SDK] Herramientas de modding desinstaladas correctamente.".into());
    }
    Ok(())
}

fn chrono_like_time() -> String {
    format!("{:?}", std::time::SystemTime::now())
}
