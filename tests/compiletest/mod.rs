use std::fs;
use std::process::Command;
use std::sync::Once;
use regex::Regex;

pub fn setup() {
    static BUILD: Once = Once::new();
    BUILD.call_once(|| {
        let status = Command::new("cargo")
            .arg("build")
            .status()
            .expect("failed to build");
        assert!(status.success());
    });
}

pub fn contains_panic(name: &str, code: &str) -> bool {
    let tempdir = tempfile::tempdir().unwrap();

    let prelude = stringify! {
        use no_panic::no_panic;
    };

    let rs = tempdir.path().join(format!("{}.rs", name));
    fs::write(&rs, format!("{}{}", prelude, code)).unwrap();

    let base_command_vec = vec!(
        "--crate-name",
        name,
        rs.to_str().unwrap(),
        "--edition=2018",
        "-C",
        "opt-level=3",
        "--emit=asm",
        "--out-dir",
        tempdir.path().to_str().unwrap(),
        // "--extern",
        // "no_panic=target/debug/libno_panic.so",
    );

    let mut lib_vec = vec!();
    let regex = Regex::new(r"^lib(.+?)-[[:alnum:]]+\.rlib").unwrap();

    for entry in std::fs::read_dir("target/debug/deps").unwrap() {
       let name = entry.unwrap().file_name().into_string().unwrap();
       if name.ends_with(".rlib") {
            lib_vec.push(String::from("--extern"));
            let lib_name = regex.captures(&name);
            let lib_name = lib_name.unwrap().get(1).map_or("", |m| m.as_str());
            lib_vec.push(format!("{}={}{}", &lib_name, "target/debug/deps/", &name));
       }
    }

    let mut arg_vec = vec!();
    arg_vec.extend(base_command_vec);
    for x in lib_vec.as_slice() {
        arg_vec.push(x);
    }
    println!("{:?}", arg_vec.join(r" "));

    let mut status = Command::new("rustc");
    let status = status.args(arg_vec);
    let status = status
        .status()
        .expect("failed to execute rustc");
    assert!(status.success());

    let asm = tempdir.path().join(format!("{}.s", name));
    let asm = fs::read_to_string(asm).unwrap();
    asm.contains("detected panic in function")
}

macro_rules! assert_no_panic {
    ($(mod $name:ident { $($content:tt)* })*) => {
        mod no_panic {
            use crate::compiletest;
            $(
                #[test]
                fn $name() {
                    compiletest::setup();
                    let name = stringify!($name);
                    let content = stringify!($($content)*);
                    assert!(!compiletest::contains_panic(name, content));
                }
            )*
        }
    };
}

macro_rules! assert_link_error {
    ($(mod $name:ident { $($content:tt)* })*) => {
        mod link_error {
            use crate::compiletest;
            $(
                #[test]
                fn $name() {
                    compiletest::setup();
                    let name = stringify!($name);
                    let content = stringify!($($content)*);
                    assert!(compiletest::contains_panic(name, content));
                }
            )*
        }
    };
}
