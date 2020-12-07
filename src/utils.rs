use log::{debug, error, info};
use quick_xml::{
    events::BytesStart,
    events::{attributes::Attributes, BytesEnd, Event},
    Error, Reader, Writer,
};
use std::{
    collections::HashMap,
    fs::{remove_dir_all, File},
    io::{Cursor, Write},
    path::Path,
    str::from_utf8,
};

use std::fs;

use crate::shell::Shell;

#[macro_export]
macro_rules! result_err {
    () => {
        |err| format!("{:?}", err)
    };
}

pub fn clone_src(
    url: &str,
    path: &str,
    branch: Option<String>,
    revision: Option<String>,
) -> Result<(), String> {
    info!("start git clone {} to {}", url, path);

    let shell = Shell::new("/tmp");
    let mut command = format!("git clone {} ", url);

    if let Some(b) = branch {
        if !b.is_empty() {
            command.push_str(&format!(" -b {} ", &b));
        }
    }

    command.push_str(path.clone());

    shell.run(&command)?;

    if let Some(commit) = revision {
        let shell = Shell::new(path);
        info!(" checkout {} ", &commit);
        let command = format!("git checkout {}", commit);
        shell.run(&command)?;
    }

    Ok(())
}

pub fn remove_dir(name: &str) {
    let f = File::open(name);

    if let Ok(_) = f {
        remove_dir_all(name).expect("删除文件失败");
        debug!("delete {} success", name);
    }
}

pub fn remove_file(name: &str) {
    let f = File::open(name);

    if let Ok(_) = f {
        fs::remove_file(name).expect("删除文件失败");
        debug!("delete {} success", name);
    }
}

pub fn file_exist(path: &str) -> bool {
    fs::metadata(path).is_ok()
}

fn has_android_name(attrs: &Attributes, meta: &HashMap<String, String>) -> bool {
    attrs.clone().into_iter().any(|s| {
        if let Ok(r) = s {
            let key = from_utf8(r.key).unwrap();
            let value = r.value.into_owned();
            let value = from_utf8(&value).unwrap();

            if key == "android:name" && meta.get(value) != None {
                return true;
            }
        }

        false
    })
}

pub fn change_xml<'a>(
    xml: &'a str,
    meta: &HashMap<String, String>,
    version_code: Option<i32>,
    version_name: Option<String>,
    path: Option<&'a str>,
) -> Result<(), Error> {
    let mut reader = Reader::from_str(xml);

    reader.trim_text(true);
    reader.expand_empty_elements(true);

    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 4);
    let mut buf = Vec::new();
    let mut add_end = false;
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) if e.name() == b"manifest" => {
                let mut elem = BytesStart::owned(b"manifest".to_vec(), "manifest".len());

                elem.extend_attributes(e.attributes().map(|attr| attr.unwrap()).filter(|s| {
                    let key = from_utf8(s.key).unwrap();
                    "android:versionName" != key && "android:versionCode" != key
                }));

                if let Some(code) = version_code {
                    elem.push_attribute(("android:versionCode", code.to_string().as_str()));
                }

                if let Some(ref s) = version_name {
                    elem.push_attribute(("android:versionName", s.as_str()));
                }

                // writes the event to the writer
                writer.write_event(Event::Start(elem))?;
            }
            Ok(Event::End(ref e)) if e.name() == b"manifest" => {
                writer.write_event(Event::End(BytesEnd::borrowed(b"manifest")))?;
            }
            Ok(Event::Start(ref e)) if e.name() == b"application" => {
                writer.write_event(Event::Start(e.clone()))?;

                if !meta.is_empty() {
                    meta.iter().for_each(|s| {
                        let mut elem = BytesStart::owned(b"meta-data".to_vec(), "meta-data".len());
                        elem.push_attribute(("android:name", s.0.as_str()));
                        elem.push_attribute(("android:value", s.1.as_str()));
                        writer.write_event(Event::Start(elem)).unwrap();
                        writer
                            .write_event(Event::End(BytesEnd::borrowed(b"meta-data")))
                            .unwrap();
                    });
                }
            }
            Ok(Event::Start(ref e)) if e.name() == b"meta-data" => {
                if !has_android_name(&e.attributes(), meta) {
                    writer.write_event(Event::Start(e.clone()))?;
                    add_end = true;
                }
            }
            Ok(Event::End(ref e)) if e.name() == b"meta-data" => {
                if add_end {
                    assert!(writer.write_event(Event::End(e.clone())).is_ok());
                    add_end = false;
                }
            }
            Ok(Event::Eof) => break,
            // you can use either `e` or `&e` if you don't want to move the event
            Ok(e) => {
                assert!(writer.write_event(&e).is_ok())
            }
            Err(e) => {
                return Err(e);
            }
        }
        buf.clear();
    }

    let result = writer.into_inner().into_inner();

    // debug!("new xml : {}", from_utf8(&result).unwrap());

    if let Some(p) = path {
        remove_file(p);

        if let Ok(mut file) = File::create(p) {
            file.write_all(&result).expect("write failed");
        } else {
            error!("create file error, {}", p);
            return Err(Error::UnexpectedEof("create file error".to_string()));
        }
    }

    Ok(())
}

#[allow(unused_must_use)]
pub fn change_properies_file(path: &str, config: &HashMap<String, String>) -> Result<(), String> {
    if !file_exist(path) {
        // 先创建parent dir
        let p = Path::new(path);
        let prefix = p.parent().unwrap();
        std::fs::create_dir_all(prefix).unwrap();
        File::create(path);
    }

    match File::open(path) {
        Ok(_) => {
            let shell = Shell::new("/tmp");

            config.iter().for_each(|f| {
                match shell.run(&format!("batcat {} | rg ^{}=", path, f.0)) {
                    Ok(_) => {
                        shell.run(&format!("sd '^{}.*' '{}={}' {}", f.0, f.0, f.1, path));
                    }
                    Err(_) => {
                        shell.run(&format!("echo \"{}={}\" >> {}", f.0, f.1, path));
                    }
                }
            });

            Ok(())
        }
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    static NAME: &'static str = "/tmp/okhttp4_demo";

    static XML: &'static str = r#"
    <?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.justsy.mdm" android:versionCode="2020010701" android:versionName="5.0.20200107r1">

    <application
        android:allowBackup="false"
        android:label="@string/main_app_name"
        android:icon="@drawable/main_icon"
        android:theme="@style/AppTheme">
        <meta-data
            android:name="brank"
            android:value="other" />
        <meta-data
            android:name="model"
            android:value="other" />
    </application>

</manifest>"#;

    #[test]
    fn properies_test() {
        crate::config::Config::get_instance();
        let mut meta: HashMap<String, String> = HashMap::new();
        meta.insert("model".to_string(), "test1".to_string());
        meta.insert("brank1".to_string(), "test2".to_string());

        assert!(super::change_properies_file("/tmp/test.prop", &meta).is_ok());
    }

    #[test]
    fn xml_manifest_test() {
        crate::config::Config::get_instance();
        let mut meta: HashMap<String, String> = HashMap::new();
        meta.insert("model".to_string(), "test2".to_string());
        meta.insert("brank".to_string(), "test2".to_string());

        assert!(super::change_xml(
            XML,
            &meta,
            Some(1111),
            Some("1.0.0".to_string()),
            Some("/tmp/tt.xml")
        )
        .is_ok());
    }

    #[test]
    fn http_clone() {
        super::remove_dir(NAME);
        let result = super::clone_src(
            "https://github.com/asmh1989/okhttp4_demo.git",
            NAME,
            None,
            None,
        );
        assert!(None == result.err());
    }

    #[test]
    fn ssh_clone() {
        super::remove_dir(NAME);
        let result = super::clone_src(
            "git@github.com:asmh1989/okhttp4_demo.git",
            NAME,
            Some("test".to_string()),
            None,
        );
        assert!(None == result.err());
    }

    #[test]
    fn ssh_clone_commit() {
        super::remove_dir(NAME);
        let result = super::clone_src(
            "git@github.com:asmh1989/okhttp4_demo.git",
            NAME,
            None,
            Some(format!("e9406d9d41cdbff36603fb0de488f09d5e18b93b")),
        );
        assert!(None == result.err());
    }
}
