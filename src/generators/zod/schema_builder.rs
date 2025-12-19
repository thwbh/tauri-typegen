use crate::generators::base::type_visitor::{TypeVisitor, ZodVisitor};
use crate::models::{TypeStructure, ValidatorAttributes};
use crate::GenerateConfig;

/// Builds complete Zod schemas including validator modifiers
pub struct ZodSchemaBuilder<'a> {
    visitor: ZodVisitor<'a>,
}

impl<'a> ZodSchemaBuilder<'a> {
    pub fn new(config: &'a GenerateConfig) -> Self {
        Self {
            visitor: ZodVisitor::with_config(config),
        }
    }

    /// Build a complete Zod schema string for a field, including validators
    pub fn build_schema(
        &self,
        type_structure: &TypeStructure,
        validator_attributes: &Option<ValidatorAttributes>,
    ) -> String {
        self.render_type(type_structure, validator_attributes, false, false)
    }

    /// Build a Zod schema for a parameter (no validators applied)
    pub fn build_param_schema(&self, type_structure: &TypeStructure) -> String {
        self.render_type(type_structure, &None, true, false)
    }

    fn render_type(
        &self,
        ts: &TypeStructure,
        validator: &Option<ValidatorAttributes>,
        skip_validation: bool,
        is_record_key: bool,
    ) -> String {
        match ts {
            TypeStructure::Optional(inner) => {
                format!(
                    "{}.optional()",
                    self.render_type(inner, validator, false, is_record_key)
                )
            }
            TypeStructure::Primitive(prim) => {
                self.render_primitive(prim, validator, skip_validation, is_record_key)
            }
            TypeStructure::Array(inner) => {
                let inner_schema = self.render_type(inner, validator, true, false);
                let array_schema = format!("z.array({})", inner_schema);
                self.apply_length_validator(&array_schema, validator, skip_validation)
            }
            TypeStructure::Map { key, value } => {
                let key_schema = self.render_type(key, validator, true, true);
                let value_schema = self.render_type(value, validator, true, false);
                format!("z.record({}, {})", key_schema, value_schema)
            }
            TypeStructure::Set(inner) => {
                let inner_schema = self.render_type(inner, validator, true, false);
                format!("z.set({})", inner_schema)
            }
            TypeStructure::Tuple(types) => {
                if types.is_empty() {
                    "z.void()".to_string()
                } else {
                    let type_strs: Vec<String> = types
                        .iter()
                        .map(|t| self.render_type(t, validator, true, false))
                        .collect();
                    format!("z.tuple([{}])", type_strs.join(", "))
                }
            }
            TypeStructure::Result(inner) => {
                let inner_schema = self.render_type(inner, validator, true, false);
                format!(
                    "z.union([{}, z.object({{ error: z.string() }})])",
                    inner_schema
                )
            }
            TypeStructure::Custom(_) => {
                // Use visitor for custom types (handles type mappings)
                self.visitor.visit_type(ts)
            }
        }
    }

    fn render_primitive(
        &self,
        type_name: &str,
        validator: &Option<ValidatorAttributes>,
        skip_validation: bool,
        is_record_key: bool,
    ) -> String {
        let base_schema = match type_name {
            "string" => {
                let schema = "z.string()".to_string();
                self.apply_string_validators(&schema, validator, skip_validation)
            }
            "number" => {
                let schema = if is_record_key {
                    "z.number()".to_string()
                } else {
                    "z.coerce.number()".to_string()
                };
                self.apply_range_validator(&schema, validator, skip_validation)
            }
            "boolean" => "z.coerce.boolean()".to_string(),
            "void" => "z.void()".to_string(),
            _ => format!("z.unknown() /* Unknown primitive: {} */", type_name),
        };
        base_schema
    }

    fn apply_string_validators(
        &self,
        schema: &str,
        validator: &Option<ValidatorAttributes>,
        skip_validation: bool,
    ) -> String {
        if skip_validation {
            return schema.to_string();
        }

        let Some(val) = validator else {
            return schema.to_string();
        };

        let mut result = schema.to_string();

        if val.email {
            result.push_str(".email()");
        }
        if val.url {
            result.push_str(".url()");
        }

        result = self.apply_length_validator(&result, validator, skip_validation);
        result
    }

    fn apply_range_validator(
        &self,
        schema: &str,
        validator: &Option<ValidatorAttributes>,
        skip_validation: bool,
    ) -> String {
        if skip_validation {
            return schema.to_string();
        }

        let Some(val) = validator else {
            return schema.to_string();
        };

        let Some(ref range) = val.range else {
            return schema.to_string();
        };

        let mut result = schema.to_string();

        if let (Some(min), Some(max)) = (range.min, range.max) {
            if let Some(ref msg) = range.message {
                result.push_str(&format!(
                    ".min({}, {{ message: \"{}\" }}).max({}, {{ message: \"{}\" }})",
                    min,
                    escape_js_string(msg),
                    max,
                    escape_js_string(msg)
                ));
            } else {
                result.push_str(&format!(".min({}).max({})", min, max));
            }
        } else if let Some(min) = range.min {
            if let Some(ref msg) = range.message {
                result.push_str(&format!(
                    ".min({}, {{ message: \"{}\" }})",
                    min,
                    escape_js_string(msg)
                ));
            } else {
                result.push_str(&format!(".min({})", min));
            }
        } else if let Some(max) = range.max {
            if let Some(ref msg) = range.message {
                result.push_str(&format!(
                    ".max({}, {{ message: \"{}\" }})",
                    max,
                    escape_js_string(msg)
                ));
            } else {
                result.push_str(&format!(".max({})", max));
            }
        }

        result
    }

    fn apply_length_validator(
        &self,
        schema: &str,
        validator: &Option<ValidatorAttributes>,
        skip_validation: bool,
    ) -> String {
        if skip_validation {
            return schema.to_string();
        }

        let Some(val) = validator else {
            return schema.to_string();
        };

        let Some(ref length) = val.length else {
            return schema.to_string();
        };

        let mut result = schema.to_string();

        if let (Some(min), Some(max)) = (length.min, length.max) {
            if let Some(ref msg) = length.message {
                result.push_str(&format!(
                    ".min({}, {{ message: \"{}\" }}).max({}, {{ message: \"{}\" }})",
                    min,
                    escape_js_string(msg),
                    max,
                    escape_js_string(msg)
                ));
            } else {
                result.push_str(&format!(".min({}).max({})", min, max));
            }
        } else if let Some(min) = length.min {
            if let Some(ref msg) = length.message {
                result.push_str(&format!(
                    ".min({}, {{ message: \"{}\" }})",
                    min,
                    escape_js_string(msg)
                ));
            } else {
                result.push_str(&format!(".min({})", min));
            }
        } else if let Some(max) = length.max {
            if let Some(ref msg) = length.message {
                result.push_str(&format!(
                    ".max({}, {{ message: \"{}\" }})",
                    max,
                    escape_js_string(msg)
                ));
            } else {
                result.push_str(&format!(".max({})", max));
            }
        }

        result
    }
}

fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
