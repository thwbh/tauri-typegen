use crate::generators::base::templates::{register_common_filters, register_common_templates};
use tera::{Context, Tera};

/// Create and configure a Tera template engine for Zod generator
pub fn create_template_engine() -> Result<Tera, String> {
    let mut tera = Tera::default();

    // Register common templates
    register_common_templates(&mut tera)?;

    // Register zod-specific templates
    register_templates(&mut tera)?;

    // Register common filters
    register_common_filters(&mut tera);

    // Register zod-specific filters
    register_filters(&mut tera);

    Ok(tera)
}

/// Register zod-specific templates from embedded strings
/// As standalone crate, tera glob features can't be used
fn register_templates(tera: &mut Tera) -> Result<(), String> {
    // Macro to reduce boilerplate for template registration
    macro_rules! template {
        ($name:expr, $path:expr) => {
            tera.add_raw_template($name, include_str!($path))
                .map_err(|e| format!("Failed to register {}: {}", $name, e))?;
        };
    }

    // Main templates
    template!("zod/types.ts.tera", "templates/types.ts.tera");
    template!("zod/commands.ts.tera", "templates/commands.ts.tera");
    template!("zod/events.ts.tera", "templates/events.ts.tera");
    template!("zod/index.ts.tera", "templates/index.ts.tera");

    // Partial templates
    template!(
        "zod/partials/schema.ts.tera",
        "templates/partials/schema.ts.tera"
    );
    template!(
        "zod/partials/field_schema.ts.tera",
        "templates/partials/field_schema.ts.tera"
    );
    template!(
        "zod/partials/enum_schema.ts.tera",
        "templates/partials/enum_schema.ts.tera"
    );
    template!(
        "zod/partials/param_schema.ts.tera",
        "templates/partials/param_schema.ts.tera"
    );
    template!(
        "zod/partials/param_schemas.ts.tera",
        "templates/partials/param_schemas.ts.tera"
    );
    template!(
        "zod/partials/type_aliases.ts.tera",
        "templates/partials/type_aliases.ts.tera"
    );
    template!(
        "zod/partials/command_function.ts.tera",
        "templates/partials/command_function.ts.tera"
    );
    template!(
        "zod/partials/event_listener.ts.tera",
        "templates/partials/event_listener.ts.tera"
    );

    Ok(())
}

/// Register zod-specific Tera filters
fn register_filters(tera: &mut Tera) {
    tera.register_filter(
        "format_return_type",
        super::filters::format_return_type_filter,
    );
    tera.register_filter("to_zod_schema", super::filters::to_zod_schema_filter);
}

/// Render a template with the given context
pub fn render(tera: &Tera, template_name: &str, context: &Context) -> Result<String, String> {
    tera.render(template_name, context).map_err(|e| {
        // Get more detailed error information
        let mut error_msg = format!("Failed to render template '{}': {}", template_name, e);

        // Check if there's a source error
        if let Some(source) = std::error::Error::source(&e) {
            error_msg.push_str(&format!("\nSource: {}", source));
        }

        error_msg
    })
}
