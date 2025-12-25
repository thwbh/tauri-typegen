use crate::{generators::base::templates::TemplateRegistry, template};
use tera::Tera;

pub struct ZodTemplate;

impl TemplateRegistry for ZodTemplate {
    /// Register zod-specific templates from embedded strings
    /// As standalone crate, tera glob features can't be used
    fn register_templates(tera: &mut Tera) -> Result<(), String> {
        // Main templates
        template!(tera, "zod/types.ts.tera", "templates/types.ts.tera");
        template!(tera, "zod/commands.ts.tera", "templates/commands.ts.tera");
        template!(tera, "zod/events.ts.tera", "templates/events.ts.tera");
        template!(tera, "zod/index.ts.tera", "templates/index.ts.tera");

        // Partial templates
        template!(
            tera,
            "zod/partials/schema.ts.tera",
            "templates/partials/schema.ts.tera"
        );

        template!(
            tera,
            "zod/partials/enum_schema.ts.tera",
            "templates/partials/enum_schema.ts.tera"
        );

        template!(
            tera,
            "zod/partials/param_schemas.ts.tera",
            "templates/partials/param_schemas.ts.tera"
        );

        template!(
            tera,
            "zod/partials/type_aliases.ts.tera",
            "templates/partials/type_aliases.ts.tera"
        );

        template!(
            tera,
            "zod/partials/command_function.ts.tera",
            "templates/partials/command_function.ts.tera"
        );
        template!(
            tera,
            "zod/partials/event_listener.ts.tera",
            "templates/partials/event_listener.ts.tera"
        );

        Ok(())
    }

    /// Register zod-specific Tera filters
    fn register_filters(tera: &mut Tera) {
        tera.register_filter("to_zod_schema", super::filters::to_zod_schema_filter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod template_registration {
        use super::*;

        #[test]
        fn test_create_tera_succeeds() {
            let result = ZodTemplate::create_tera();
            assert!(result.is_ok());
        }

        #[test]
        fn test_has_main_templates() {
            let tera = ZodTemplate::create_tera().unwrap();
            let template_names: Vec<&str> = tera.get_template_names().collect();

            assert!(template_names.contains(&"zod/types.ts.tera"));
            assert!(template_names.contains(&"zod/commands.ts.tera"));
            assert!(template_names.contains(&"zod/events.ts.tera"));
            assert!(template_names.contains(&"zod/index.ts.tera"));
        }

        #[test]
        fn test_has_partial_templates() {
            let tera = ZodTemplate::create_tera().unwrap();
            let template_names: Vec<&str> = tera.get_template_names().collect();

            assert!(template_names.contains(&"zod/partials/schema.ts.tera"));
            assert!(template_names.contains(&"zod/partials/enum_schema.ts.tera"));
            assert!(template_names.contains(&"zod/partials/param_schemas.ts.tera"));
            assert!(template_names.contains(&"zod/partials/type_aliases.ts.tera"));
            assert!(template_names.contains(&"zod/partials/command_function.ts.tera"));
            assert!(template_names.contains(&"zod/partials/event_listener.ts.tera"));
        }

        #[test]
        fn test_has_common_template() {
            let tera = ZodTemplate::create_tera().unwrap();
            let template_names: Vec<&str> = tera.get_template_names().collect();

            assert!(template_names.contains(&"common/header.tera"));
        }

        #[test]
        fn test_template_count() {
            let tera = ZodTemplate::create_tera().unwrap();
            let count = tera.get_template_names().count();
            // Should have 11 templates (4 main + 6 partials + 1 common)
            assert!(count == 11);
        }

        #[test]
        fn test_has_common_filters() {
            let tera = ZodTemplate::create_tera().unwrap();

            // Test that common filters are registered
            assert!(tera.get_filter("escape_js").is_ok());
            assert!(tera.get_filter("add_types_prefix").is_ok());
        }

        #[test]
        fn test_has_zod_filter() {
            let tera = ZodTemplate::create_tera().unwrap();

            // Test that zod-specific filter is registered
            assert!(tera.get_filter("to_zod_schema").is_ok());
        }
    }

    mod filter_registration {
        use super::*;

        #[test]
        fn test_register_filters_adds_to_zod_schema() {
            let mut tera = tera::Tera::default();
            ZodTemplate::register_filters(&mut tera);

            // Should add the to_zod_schema filter
            assert!(tera.get_filter("to_zod_schema").is_ok());
        }
    }
}
