use crate::generators::base::templates::{register_common_filters, register_common_templates};
use tera::{Context, Tera};

/// Create and configure a Tera template engine for TypeScript generator
pub fn create_template_engine() -> Result<Tera, String> {
    let mut tera = Tera::default();

    // Register common templates
    register_common_templates(&mut tera)?;

    // Register typescript-specific templates (if any)
    register_templates(&mut tera)?;

    // Register common filters
    register_common_filters(&mut tera);

    // No typescript-specific filters currently needed

    Ok(tera)
}

/// Register typescript-specific templates from embedded strings
fn register_templates(tera: &mut Tera) -> Result<(), String> {
    // Macro to reduce boilerplate for template registration
    macro_rules! template {
        ($name:expr, $path:expr) => {
            tera.add_raw_template($name, include_str!($path))
                .map_err(|e| format!("Failed to register {}: {}", $name, e))?;
        };
    }

    // Main templates
    template!("typescript/types.ts.tera", "templates/types.ts.tera");
    template!("typescript/commands.ts.tera", "templates/commands.ts.tera");
    template!("typescript/events.ts.tera", "templates/events.ts.tera");
    template!("typescript/index.ts.tera", "templates/index.ts.tera");

    // Partial templates
    template!(
        "typescript/partials/interface.tera",
        "templates/partials/interface.tera"
    );
    template!(
        "typescript/partials/enum.tera",
        "templates/partials/enum.tera"
    );
    template!(
        "typescript/partials/param_interface.ts.tera",
        "templates/partials/param_interface.ts.tera"
    );
    template!(
        "typescript/partials/command_function.ts.tera",
        "templates/partials/command_function.ts.tera"
    );
    template!(
        "typescript/partials/event_listener.ts.tera",
        "templates/partials/event_listener.ts.tera"
    );

    Ok(())
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
