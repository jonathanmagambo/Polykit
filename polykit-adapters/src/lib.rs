pub mod go;
pub mod js;
pub mod python;
pub mod rust;

pub use go::GoAdapter;
pub use js::JsAdapter;
pub use python::PythonAdapter;
pub use rust::RustAdapter;

use polykit_core::adapter::LanguageAdapter;
use polykit_core::package::Language;

pub fn get_adapter(language: &Language) -> Box<dyn LanguageAdapter> {
    match language {
        Language::Js | Language::Ts => Box::new(JsAdapter),
        Language::Python => Box::new(PythonAdapter),
        Language::Go => Box::new(GoAdapter),
        Language::Rust => Box::new(RustAdapter),
    }
}
