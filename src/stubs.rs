use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use hlbc::types::{FunPtr, Type, TypeObj};
use hlbc::{Bytecode, Resolve};

#[derive(Debug, Clone, Default)]
pub struct StubsReport {
    pub classes_count: usize,
    pub files_written: usize,
    pub out_dir: PathBuf,
}

/// Convierte un tipo HashLink a su representación en sintaxis Haxe.
pub fn hl_type_to_haxe(code: &Bytecode, t: &Type) -> String {
    match t {
        Type::Void => "Void".to_string(),
        Type::UI8 => "Int".to_string(),
        Type::UI16 => "Int".to_string(),
        Type::I32 => "Int".to_string(),
        Type::I64 => "haxe.Int64".to_string(),
        Type::F32 => "Single".to_string(),
        Type::F64 => "Float".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::Bytes => "hl.Bytes".to_string(),
        Type::Dyn => "Dynamic".to_string(),
        Type::Fun(f) => {
            let args: Vec<String> = f
                .args
                .iter()
                .map(|a| hl_type_to_haxe(code, &code[*a]))
                .collect();
            let ret = hl_type_to_haxe(code, &code[f.ret]);
            if args.is_empty() {
                format!("()->{}", ret)
            } else {
                format!("({})->{}", args.join(", "), ret)
            }
        }
        Type::Obj(o) => {
            let name = &code[o.name];
            clean_type_name(name)
        }
        Type::Array => "hl.NativeArray<Dynamic>".to_string(),
        Type::Type => "hl.Type".to_string(),
        Type::Ref(r) => format!("hl.Ref<{}>", hl_type_to_haxe(code, &code[*r])),
        Type::Virtual { .. } => "Dynamic".to_string(),
        Type::DynObj => "Dynamic".to_string(),
        Type::Abstract { name } => clean_type_name(&code[*name]),
        Type::Enum { name, .. } => clean_type_name(&code[*name]),
        Type::Null(r) => format!("Null<{}>", hl_type_to_haxe(code, &code[*r])),
        Type::Method(f) => {
            let ret = hl_type_to_haxe(code, &code[f.ret]);
            format!("Dynamic /* Method: {} */", ret)
        }
        Type::Struct(s) => clean_type_name(&code[s.name]),
        Type::Packed(r) => hl_type_to_haxe(code, &code[*r]),
        Type::Guid => "hl.Guid".to_string(),
    }
}

pub fn clean_type_name(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return "Dynamic".to_string();
    }
    // Quitar prefijo $ de clases estáticas de Haxe y normalizar separadores
    let name = raw.strip_prefix('$').unwrap_or(raw);
    let name = name.replace(".$", ".").replace('$', ".");
    if name == "String" {
        return "String".to_string();
    }
    if name == "Array" {
        return "Array<Dynamic>".to_string();
    }
    if name == "Date" {
        return "Date".to_string();
    }
    if name == "Math" {
        return "Math".to_string();
    }
    if name.starts_with('~') || name.contains('#') || name.contains('<') {
        return "Dynamic".to_string();
    }
    name
}

fn is_internal_field(fname: &str) -> bool {
    fname.is_empty()
        || fname.starts_with('~')
        || fname == "__name__"
        || fname == "__constructor__"
        || fname == "__type__"
        || fname == "__meta__"
        || fname == "__implementedBy__"
        || fname == "__classes__"
}

pub fn find_static_companion<'a>(code: &'a Bytecode, full_name: &str) -> Option<&'a TypeObj> {
    for t in &code.types {
        if let Type::Obj(obj) = t {
            let raw = &code[obj.name];
            if raw.contains('$') && !raw.starts_with('~') && clean_type_name(raw) == full_name {
                return Some(obj);
            }
        }
    }
    None
}

/// Genera el archivo .hx con la definición `extern class` para un objeto HL.
pub fn generate_class_stub(
    code: &Bytecode,
    obj: &TypeObj,
) -> Option<(String, String, String)> {
    let full_name = clean_type_name(&code[obj.name]);
    let companion = find_static_companion(code, &full_name);
    generate_class_stub_with_companion(code, obj, companion)
}

/// Versión con companion estático precalculado para procesamiento en lote óptimo.
pub fn generate_class_stub_with_companion(
    code: &Bytecode,
    obj: &TypeObj,
    static_companion: Option<&TypeObj>,
) -> Option<(String, String, String)> {
    let raw_name = &code[obj.name];
    // Ignorar clases internas anónimas/closures y los objetos estáticos directos (se integran en la clase base)
    if raw_name.starts_with('~') || raw_name.contains('$') || raw_name.is_empty() {
        return None;
    }

    let full_name = clean_type_name(raw_name);
    let parts: Vec<&str> = full_name.split('.').collect();
    let class_name = *parts.last()?;
    if class_name.is_empty() || (class_name.starts_with('_') && parts.len() == 1) {
        // Ignorar tipos internos especiales
    }

    let package_name = if parts.len() > 1 {
        parts[..parts.len() - 1].join(".")
    } else {
        String::new()
    };

    let mut out = String::new();
    out.push_str("package");
    if !package_name.is_empty() {
        out.push(' ');
        out.push_str(&package_name);
    }
    out.push_str(";\n\n");
    out.push_str("/**\n");
    out.push_str(" * Generado automaticamente por HashToolkit para HashLoader.\n");
    out.push_str(&format!(" * Clase original: {}\n", full_name));
    out.push_str(" */\n");

    let super_str = obj
        .super_
        .as_ref()
        .and_then(|s| {
            let st = &code[*s];
            if let Type::Obj(so) = st {
                let sn = clean_type_name(&code[so.name]);
                if !sn.is_empty() && sn != "Dynamic" {
                    Some(format!(" extends {}", sn))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or_default();

    out.push_str(&format!("extern class {}{} {{\n", class_name, super_str));

    // 1. Campos estáticos (si existe el companion `$ClassName`)
    if let Some(st_obj) = static_companion {
        for field in &st_obj.fields {
            let fname = &code[field.name];
            if is_internal_field(fname) {
                continue;
            }
            let ftype = &code[field.t];
            if let Type::Fun(f) = ftype {
                let mut arg_strs = Vec::new();
                for (idx, arg) in f.args.iter().enumerate() {
                    let atype = hl_type_to_haxe(code, &code[*arg]);
                    arg_strs.push(format!("arg{}:{}", idx, atype));
                }
                let ret = hl_type_to_haxe(code, &code[f.ret]);
                out.push_str(&format!(
                    "\tpublic static function {}({}):{};\n",
                    fname,
                    arg_strs.join(", "),
                    ret
                ));
            } else {
                let ftype_str = hl_type_to_haxe(code, ftype);
                out.push_str(&format!("\tpublic static var {}:{};\n", fname, ftype_str));
            }
        }

        // Métodos estáticos definidos en protos (si hubiese)
        for proto in &st_obj.protos {
            let mname = &code[proto.name];
            if is_internal_field(mname) {
                continue;
            }
            let fun_type_ref = match code.get(proto.findex) {
                FunPtr::Fun(f) => f.t,
                FunPtr::Native(n) => n.t,
            };
            if let Type::Fun(f) = &code[fun_type_ref] {
                let mut arg_strs = Vec::new();
                for (idx, arg) in f.args.iter().enumerate() {
                    let atype = hl_type_to_haxe(code, &code[*arg]);
                    arg_strs.push(format!("arg{}:{}", idx, atype));
                }
                let ret = hl_type_to_haxe(code, &code[f.ret]);
                out.push_str(&format!(
                    "\tpublic static function {}({}):{};\n",
                    mname,
                    arg_strs.join(", "),
                    ret
                ));
            }
        }

        if !st_obj.fields.is_empty() || !st_obj.protos.is_empty() {
            out.push('\n');
        }
    }

    // 2. Campos de instancia
    for field in &obj.fields {
        let fname = &code[field.name];
        if is_internal_field(fname) {
            continue;
        }
        let ftype = hl_type_to_haxe(code, &code[field.t]);
        out.push_str(&format!("\tpublic var {}:{};\n", fname, ftype));
    }

    if !obj.fields.is_empty() && !obj.protos.is_empty() {
        out.push('\n');
    }

    // 3. Métodos de instancia
    for proto in &obj.protos {
        let mname = &code[proto.name];
        if is_internal_field(mname) && mname != "__constructor__" {
            continue;
        }
        let fun_type_ref = match code.get(proto.findex) {
            FunPtr::Fun(f) => f.t,
            FunPtr::Native(n) => n.t,
        };
        if let Type::Fun(f) = &code[fun_type_ref] {
            let mut arg_strs = Vec::new();
            // Para métodos de instancia, el primer argumento es `this` (p0)
            let skip_first = !f.args.is_empty();
            let args_to_use = if skip_first { &f.args[1..] } else { &f.args[..] };
            for (idx, arg) in args_to_use.iter().enumerate() {
                let atype = hl_type_to_haxe(code, &code[*arg]);
                arg_strs.push(format!("arg{}:{}", idx, atype));
            }
            let ret = hl_type_to_haxe(code, &code[f.ret]);
            if mname == "__constructor__" || mname == "new" {
                out.push_str(&format!("\tpublic function new({});\n", arg_strs.join(", ")));
            } else {
                out.push_str(&format!(
                    "\tpublic function {}({}):{};\n",
                    mname,
                    arg_strs.join(", "),
                    ret
                ));
            }
        }
    }

    out.push_str("}\n");
    Some((package_name, class_name.to_string(), out))
}

/// Genera el árbol completo de stubs `.hx` a partir de un archivo bytecode `.hl`.
pub fn generate_all_stubs(hl_path: &Path, out_dir: &Path) -> Result<StubsReport, String> {
    let code = Bytecode::from_file(hl_path).map_err(|e| format!("Error cargando bytecode: {e}"))?;
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;

    let mut report = StubsReport {
        classes_count: 0,
        files_written: 0,
        out_dir: out_dir.to_path_buf(),
    };

    // Mapear objetos estáticos ($ClassName) para integrarlos a sus clases correspondientes
    let mut static_map: HashMap<String, &TypeObj> = HashMap::new();
    for t in &code.types {
        if let Type::Obj(obj) = t {
            let raw_name = &code[obj.name];
            if raw_name.contains('$') && !raw_name.starts_with('~') {
                let cleaned = clean_type_name(raw_name);
                static_map.insert(cleaned, obj);
            }
        }
    }

    for t in &code.types {
        if let Type::Obj(obj) = t {
            report.classes_count += 1;
            let full_name = clean_type_name(&code[obj.name]);
            let static_companion = static_map.get(&full_name).copied();

            if let Some((pkg, class_name, content)) = generate_class_stub_with_companion(&code, obj, static_companion) {
                let mut dir = out_dir.to_path_buf();
                if !pkg.is_empty() {
                    for part in pkg.split('.') {
                        dir = dir.join(part);
                    }
                    let _ = fs::create_dir_all(&dir);
                }
                let file_path = dir.join(format!("{}.hx", class_name));
                if fs::write(&file_path, content).is_ok() {
                    report.files_written += 1;
                }
            }
        }
    }

    // Generar archivo README para el modder
    let readme = format!(
        r#"# HashLoader SDK — Stubs para Haxe
=====================================
Generado automáticamente desde: {}
Clases analizadas: {}
Archivos generados: {}

Cómo usar estos stubs en tu mod:
1. En tu archivo 'build.hxml', agrega una línea con la ruta a esta carpeta:
   -cp {}
2. En tus archivos .hx de mod (por ejemplo src/Main.hx), podrás referenciar
   cualquier clase del juego con tipado estricto y autocompletado en VS Code:
   import PlayState;
   import flixel.FlxSprite;

— Generado por HashToolkit (HashLoader Modding SDK)
"#,
        hl_path.display(),
        report.classes_count,
        report.files_written,
        out_dir.display()
    );
    let _ = fs::write(out_dir.join("README.txt"), readme);

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terex_stubs_integrity() {
        let hl_path = Path::new(r"C:\Users\Jn\Desktop\compilación\Terex-master\build\main.hl");
        if !hl_path.is_file() {
            return;
        }
        let out_dir = std::env::temp_dir().join("hashloader_test_stubs");
        let _ = fs::remove_dir_all(&out_dir);
        let report = generate_all_stubs(hl_path, &out_dir).expect("stubs generation failed");
        assert!(report.files_written > 0);

        // 1. Verificar Player.hx
        let player_stub = fs::read_to_string(out_dir.join("Player.hx")).expect("missing Player.hx");
        assert!(player_stub.contains("class Player"), "Player stub must define class Player");
        assert!(
            player_stub.contains("public function update(arg0:Float, arg1:Int):Void;"),
            "Player.update signature must be (Float, Int):Void, found:\n{player_stub}"
        );

        // 2. Verificar Main.hx
        let main_stub = fs::read_to_string(out_dir.join("Main.hx")).expect("missing Main.hx");
        assert!(
            main_stub.contains("public function init():Void;"),
            "Main.init must return Void, found:\n{main_stub}"
        );

        // 3. Verificar hxd/System.hx con métodos estáticos
        let system_stub = fs::read_to_string(out_dir.join("hxd").join("System.hx")).expect("missing System.hx");
        assert!(
            system_stub.contains("public static function getCurrentLoop"),
            "hxd.System must contain public static function getCurrentLoop, found:\n{system_stub}"
        );

        let _ = fs::remove_dir_all(&out_dir);
    }
}
