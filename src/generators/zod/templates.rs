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
            "zod/partials/field_schema.ts.tera",
            "templates/partials/field_schema.ts.tera"
        );
        template!(
            tera,
            "zod/partials/enum_schema.ts.tera",
            "templates/partials/enum_schema.ts.tera"
        );
        template!(
            tera,
            "zod/partials/param_schema.ts.tera",
            "templates/partials/param_schema.ts.tera"
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
