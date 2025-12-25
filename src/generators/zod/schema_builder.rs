use crate::generators::base::type_visitor::TypeVisitor;
use crate::generators::zod::type_visitor::ZodVisitor;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LengthConstraint, RangeConstraint};

    fn test_config() -> GenerateConfig {
        GenerateConfig::default()
    }

    #[test]
    fn test_build_schema_primitives() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let ts = TypeStructure::Primitive("string".to_string());
        assert_eq!(builder.build_schema(&ts, &None), "z.string()");

        let ts = TypeStructure::Primitive("number".to_string());
        assert_eq!(builder.build_schema(&ts, &None), "z.coerce.number()");

        let ts = TypeStructure::Primitive("boolean".to_string());
        assert_eq!(builder.build_schema(&ts, &None), "z.coerce.boolean()");

        let ts = TypeStructure::Primitive("void".to_string());
        assert_eq!(builder.build_schema(&ts, &None), "z.void()");
    }

    #[test]
    fn test_build_schema_optional() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let ts = TypeStructure::Optional(Box::new(TypeStructure::Primitive("string".to_string())));
        assert_eq!(builder.build_schema(&ts, &None), "z.string().optional()");
    }

    #[test]
    fn test_build_schema_array() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let ts = TypeStructure::Array(Box::new(TypeStructure::Primitive("string".to_string())));
        assert_eq!(builder.build_schema(&ts, &None), "z.array(z.string())");
    }

    #[test]
    fn test_build_schema_with_length_validator() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let validator = ValidatorAttributes {
            email: false,
            url: false,
            length: Some(LengthConstraint {
                min: Some(5),
                max: Some(10),
                message: None,
            }),
            range: None,
            custom_message: None,
        };

        let ts = TypeStructure::Primitive("string".to_string());
        let result = builder.build_schema(&ts, &Some(validator));
        assert!(result.contains(".min(5)"));
        assert!(result.contains(".max(10)"));
    }

    #[test]
    fn test_build_schema_with_email_validator() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let validator = ValidatorAttributes {
            email: true,
            url: false,
            length: None,
            range: None,
            custom_message: None,
        };

        let ts = TypeStructure::Primitive("string".to_string());
        let result = builder.build_schema(&ts, &Some(validator));
        assert!(result.contains(".email()"));
    }

    #[test]
    fn test_build_schema_with_url_validator() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let validator = ValidatorAttributes {
            email: false,
            url: true,
            length: None,
            range: None,
            custom_message: None,
        };

        let ts = TypeStructure::Primitive("string".to_string());
        let result = builder.build_schema(&ts, &Some(validator));
        assert!(result.contains(".url()"));
    }

    #[test]
    fn test_build_schema_with_range_validator() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let validator = ValidatorAttributes {
            email: false,
            url: false,
            length: None,
            range: Some(RangeConstraint {
                min: Some(1.0),
                max: Some(100.0),
                message: None,
            }),
            custom_message: None,
        };

        let ts = TypeStructure::Primitive("number".to_string());
        let result = builder.build_schema(&ts, &Some(validator));
        assert!(result.contains(".min(1)"));
        assert!(result.contains(".max(100)"));
    }

    #[test]
    fn test_build_schema_with_validator_message() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let validator = ValidatorAttributes {
            email: false,
            url: false,
            length: Some(LengthConstraint {
                min: Some(3),
                max: None,
                message: Some("Too short".to_string()),
            }),
            range: None,
            custom_message: None,
        };

        let ts = TypeStructure::Primitive("string".to_string());
        let result = builder.build_schema(&ts, &Some(validator));
        assert!(result.contains("Too short"));
    }

    #[test]
    fn test_build_schema_map() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let ts = TypeStructure::Map {
            key: Box::new(TypeStructure::Primitive("string".to_string())),
            value: Box::new(TypeStructure::Primitive("number".to_string())),
        };
        assert_eq!(
            builder.build_schema(&ts, &None),
            "z.record(z.string(), z.coerce.number())"
        );
    }

    #[test]
    fn test_build_schema_set() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let ts = TypeStructure::Set(Box::new(TypeStructure::Primitive("string".to_string())));
        assert_eq!(builder.build_schema(&ts, &None), "z.set(z.string())");
    }

    #[test]
    fn test_build_schema_tuple() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let ts = TypeStructure::Tuple(vec![
            TypeStructure::Primitive("string".to_string()),
            TypeStructure::Primitive("number".to_string()),
        ]);
        assert_eq!(
            builder.build_schema(&ts, &None),
            "z.tuple([z.string(), z.coerce.number()])"
        );
    }

    #[test]
    fn test_build_schema_empty_tuple() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let ts = TypeStructure::Tuple(vec![]);
        assert_eq!(builder.build_schema(&ts, &None), "z.void()");
    }

    #[test]
    fn test_build_schema_result() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let ts = TypeStructure::Result(Box::new(TypeStructure::Primitive("string".to_string())));
        assert_eq!(
            builder.build_schema(&ts, &None),
            "z.union([z.string(), z.object({ error: z.string() })])"
        );
    }

    #[test]
    fn test_build_schema_custom() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let ts = TypeStructure::Custom("User".to_string());
        assert_eq!(builder.build_schema(&ts, &None), "UserSchema");
    }

    #[test]
    fn test_build_param_schema() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let ts = TypeStructure::Primitive("string".to_string());
        assert_eq!(builder.build_param_schema(&ts), "z.string()");
    }

    #[test]
    fn test_build_param_schema_no_validation() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        // Even with validator, param schema should not apply validation
        let ts = TypeStructure::Array(Box::new(TypeStructure::Primitive("string".to_string())));
        assert_eq!(builder.build_param_schema(&ts), "z.array(z.string())");
    }

    #[test]
    fn test_escape_js_string() {
        assert_eq!(escape_js_string("hello"), "hello");
        assert_eq!(escape_js_string("hello\\world"), "hello\\\\world");
        assert_eq!(escape_js_string("hello\"world"), "hello\\\"world");
        assert_eq!(escape_js_string("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_js_string("hello\rworld"), "hello\\rworld");
        assert_eq!(escape_js_string("hello\tworld"), "hello\\tworld");
    }

    #[test]
    fn test_array_length_validator() {
        let config = test_config();
        let builder = ZodSchemaBuilder::new(&config);

        let validator = ValidatorAttributes {
            email: false,
            url: false,
            length: Some(LengthConstraint {
                min: Some(2),
                max: Some(5),
                message: None,
            }),
            range: None,
            custom_message: None,
        };

        let ts = TypeStructure::Array(Box::new(TypeStructure::Primitive("string".to_string())));
        let result = builder.build_schema(&ts, &Some(validator));
        assert!(result.contains("z.array"));
        assert!(result.contains(".min(2)"));
        assert!(result.contains(".max(5)"));
    }
}
