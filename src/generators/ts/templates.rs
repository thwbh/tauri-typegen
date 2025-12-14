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
