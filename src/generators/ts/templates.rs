use crate::{generators::base::templates::TemplateRegistry, template};
use tera::Tera;

pub struct TypeScriptTemplate;

/// Create and configure a Tera template engine for TypeScript generator
impl TemplateRegistry for TypeScriptTemplate {
    fn register_filters(_tera: &mut Tera) {}

    /// Register typescript-specific templates from embedded strings
    fn register_templates(tera: &mut Tera) -> Result<(), String> {
        // Main templates
        template!(tera, "typescript/types.ts.tera", "templates/types.ts.tera");
        template!(
            tera,
            "typescript/commands.ts.tera",
            "templates/commands.ts.tera"
        );
        template!(
            tera,
            "typescript/events.ts.tera",
            "templates/events.ts.tera"
        );
        template!(tera, "typescript/index.ts.tera", "templates/index.ts.tera");

        // Partial templates
        template!(
            tera,
            "typescript/partials/interface.tera",
            "templates/partials/interface.tera"
        );
        template!(
            tera,
            "typescript/partials/enum.tera",
            "templates/partials/enum.tera"
        );
        template!(
            tera,
            "typescript/partials/param_interface.ts.tera",
            "templates/partials/param_interface.ts.tera"
        );
        template!(
            tera,
            "typescript/partials/command_function.ts.tera",
            "templates/partials/command_function.ts.tera"
        );
        template!(
            tera,
            "typescript/partials/event_listener.ts.tera",
            "templates/partials/event_listener.ts.tera"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod template_registration {
        use super::*;

        #[test]
        fn test_create_tera_succeeds() {
            let result = TypeScriptTemplate::create_tera();
            assert!(result.is_ok());
        }

        #[test]
        fn test_has_main_templates() {
            let tera = TypeScriptTemplate::create_tera().unwrap();
            let template_names: Vec<&str> = tera.get_template_names().collect();

            assert!(template_names.contains(&"typescript/types.ts.tera"));
            assert!(template_names.contains(&"typescript/commands.ts.tera"));
            assert!(template_names.contains(&"typescript/events.ts.tera"));
            assert!(template_names.contains(&"typescript/index.ts.tera"));
        }

        #[test]
        fn test_has_partial_templates() {
            let tera = TypeScriptTemplate::create_tera().unwrap();
            let template_names: Vec<&str> = tera.get_template_names().collect();

            assert!(template_names.contains(&"typescript/partials/interface.tera"));
            assert!(template_names.contains(&"typescript/partials/enum.tera"));
            assert!(template_names.contains(&"typescript/partials/param_interface.ts.tera"));
            assert!(template_names.contains(&"typescript/partials/command_function.ts.tera"));
            assert!(template_names.contains(&"typescript/partials/event_listener.ts.tera"));
        }

        #[test]
        fn test_has_common_template() {
            let tera = TypeScriptTemplate::create_tera().unwrap();
            let template_names: Vec<&str> = tera.get_template_names().collect();

            assert!(template_names.contains(&"common/header.tera"));
        }

        #[test]
        fn test_template_count() {
            let tera = TypeScriptTemplate::create_tera().unwrap();
            let count = tera.get_template_names().count();
            // Should have at least 10 templates (4 main + 5 partials + 1 common)
            assert!(count >= 10);
        }

        #[test]
        fn test_has_common_filters() {
            let tera = TypeScriptTemplate::create_tera().unwrap();

            // Test that common filters are registered
            assert!(tera.get_filter("escape_js").is_ok());
            assert!(tera.get_filter("add_types_prefix").is_ok());
        }
    }

    mod filter_registration {
        use super::*;

        #[test]
        fn test_register_filters_no_typescript_specific_filters() {
            // TypeScript template doesn't add any custom filters beyond the common ones
            let mut tera = tera::Tera::default();
            TypeScriptTemplate::register_filters(&mut tera);

            // TypeScript doesn't register any custom filters (only common filters in create_tera)
            // Just verify it doesn't panic
            assert!(tera.get_filter("escape_js").is_err());
        }
    }
}
