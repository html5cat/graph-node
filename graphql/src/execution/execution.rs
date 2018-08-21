use graphql_parser::query as q;
use graphql_parser::schema as s;
use indexmap::IndexMap;
use std::cmp;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::fmt;

use graph::prelude::*;

use prelude::*;
use query::ast as qast;
use schema::ast as sast;

/// GraphQL execution.
#[derive(Clone)]
pub struct Execution<'a, R1, R2>
where
    R1: Resolver,
    R2: Resolver,
{
    /// The logger to use.
    pub logger: slog::Logger,
    /// The schema to execute the query against.
    pub schema: &'a s::Document,
    /// Introspection data that corresponds to the schema.
    pub introspection_schema: &'a s::Document,
    /// The query/subscription document to execute.
    pub document: &'a q::Document,
    /// The resolver to use.
    pub resolver: Arc<R1>,
    /// The introspection resolver to use.
    pub introspection_resolver: Arc<R2>,
    /// The current field stack (e.g. allUsers > friends > name).
    pub fields: Vec<q::Field>,
    /// Whether or not we're executing an introspection query
    pub introspecting: bool,
    /// Errors that occured during the execution.
    pub errors: Vec<ExecutionError>,
}

impl<'a, R1, R2> Execution<'a, R1, R2>
where
    R1: Resolver,
    R2: Resolver,
{
    /// Executes a selection set, requiring the result to be of the given object type.
    ///
    /// Allows passing in a parent value during recursive processing of objects and their fields.
    pub fn execute_selection_set(
        &mut self,
        selection_set: &q::SelectionSet,
        object_type: &'a s::ObjectType,
        object_value: &Option<q::Value>,
    ) -> q::Value {
        let mut result_map: BTreeMap<String, q::Value> = BTreeMap::new();

        // Group fields with the same response key, so we can execute them together
        let grouped_field_set = self.collect_fields(object_type, selection_set, None);

        // Process all field groups in order
        for (response_key, fields) in grouped_field_set {
            // If the field exists on the object, execute it and add its result to the result map
            if let Some((ref field_def, introspecting)) =
                self.get_field_type(object_type, &fields[0].name)
            {
                // Push the new field onto the context's field stack
                self.fields.push(fields[0].clone());

                // Remember whether or not we're introspecting now
                self.introspecting = introspecting;

                match self.execute_field(object_type, object_value, &fields[0], field_def, &fields)
                {
                    Ok(v) => {
                        result_map.insert(response_key.to_owned(), v);
                    }
                    Err(e) => {
                        result_map.insert(response_key.to_owned(), q::Value::Null);
                        self.errors.push(e);
                    }
                };

                // Pop the field off again
                self.fields.pop();
            }
        }

        // If we have result data, wrap it in an output object
        if result_map.is_empty() {
            q::Value::Null
        } else {
            q::Value::Object(result_map)
        }
    }

    /// Collects fields of a selection set.
    fn collect_fields(
        &mut self,
        object_type: &'a s::ObjectType,
        selection_set: &q::SelectionSet,
        visited_fragments: Option<HashSet<q::Name>>,
    ) -> IndexMap<String, Vec<q::Field>> {
        let mut visited_fragments = visited_fragments.unwrap_or(HashSet::new());
        let mut grouped_fields = IndexMap::new();

        // Only consider selections that are not skipped and should be included
        let selections: Vec<_> = selection_set
            .items
            .iter()
            .filter(|selection| !qast::skip_selection(selection))
            .filter(|selection| qast::include_selection(selection))
            .collect();

        for selection in selections {
            match selection {
                q::Selection::Field(field) => {
                    // Obtain the response key for the field
                    let response_key = qast::get_response_key(&field);

                    // Create a field group for this response key on demand
                    if !grouped_fields.contains_key(response_key) {
                        grouped_fields.insert(response_key.to_owned(), vec![]);
                    }

                    // Append the selection field to this group
                    let mut group = grouped_fields.get_mut(response_key).unwrap();
                    group.push(field.clone());
                }

                q::Selection::FragmentSpread(spread) => {
                    // Only consider the fragment if it hasn't already been included,
                    // as would be the case if the same fragment spread ...Foo appeared
                    // twice in the same selection set
                    if !visited_fragments.contains(&spread.fragment_name) {
                        visited_fragments.insert(spread.fragment_name.clone());

                        // Resolve the fragment using its name and, if it applies, collect
                        // fields for the fragment and group them
                        let fragment_grouped_field_set = qast::get_fragment(
                            self.document,
                            &spread.fragment_name,
                        ).and_then(|fragment| {
                            // We have a fragment, only pass it on if it applies to the
                            // current object type
                            if self.does_fragment_type_apply(object_type, &fragment.type_condition)
                            {
                                Some(fragment)
                            } else {
                                None
                            }
                        })
                            .map(|fragment| {
                                // We have a fragment that applies to the current object type,
                                // collect its fields into response key groups
                                self.collect_fields(
                                    object_type,
                                    &fragment.selection_set,
                                    Some(visited_fragments.clone()),
                                )
                            });

                        if let Some(grouped_field_set) = fragment_grouped_field_set {
                            // Add all items from each fragments group to the field group
                            // with the corresponding response key
                            for (response_key, mut fragment_group) in grouped_field_set {
                                grouped_fields
                                    .entry(response_key)
                                    .or_insert(vec![])
                                    .append(&mut fragment_group);
                            }
                        }
                    }
                }

                q::Selection::InlineFragment(_) => unimplemented!(),
            };
        }

        grouped_fields
    }

    /// Determines whether a fragment is applicable to the given object type.
    fn does_fragment_type_apply(
        &mut self,
        object_type: &s::ObjectType,
        fragment_type: &q::TypeCondition,
    ) -> bool {
        // This is safe to do, as TypeCondition only has a single `On` variant.
        let q::TypeCondition::On(ref name) = fragment_type;

        // Resolve the type the fragment applies to based on its name
        let named_type = sast::get_named_type(
            if self.introspecting {
                self.introspection_schema
            } else {
                &self.schema
            },
            name,
        );

        match named_type {
            // The fragment applies to the object type if its type is the same object type
            Some(s::TypeDefinition::Object(ot)) => object_type == ot,

            // The fragment also applies to the object type if its type is an interface
            // that the object type implements
            Some(s::TypeDefinition::Interface(it)) => object_type
                .implements_interfaces
                .iter()
                .find(|name| name == &&it.name)
                .map(|_| true)
                .unwrap_or(false),

            // The fragment also applies to an object type if its type is a union that
            // the object type is one of the possible types for
            Some(s::TypeDefinition::Union(ut)) => ut
                .types
                .iter()
                .find(|name| name == &&object_type.name)
                .map(|_| true)
                .unwrap_or(false),

            // In all other cases, the fragment does not apply
            _ => false,
        }
    }

    /// Executes a field.
    fn execute_field(
        &mut self,
        object_type: &'a s::ObjectType,
        object_value: &Option<q::Value>,
        field: &q::Field,
        field_definition: &'a s::Field,
        fields: &Vec<q::Field>,
    ) -> Result<q::Value, ExecutionError> {
        self.coerce_argument_values(object_type, field)
            .and_then(|argument_values| {
                self.resolve_field_value(
                    object_type,
                    object_value,
                    field,
                    field_definition,
                    &field_definition.field_type,
                    &argument_values,
                )
            })
            .and_then(|value| {
                self.complete_value(field, &field_definition.field_type, fields, value)
            })
    }

    /// Resolves the value of a field.
    fn resolve_field_value(
        &mut self,
        object_type: &s::ObjectType,
        object_value: &Option<q::Value>,
        field: &q::Field,
        field_definition: &s::Field,
        field_type: &s::Type,
        argument_values: &HashMap<&q::Name, q::Value>,
    ) -> Result<q::Value, ExecutionError> {
        match field_type {
            s::Type::NonNullType(inner_type) => self.resolve_field_value(
                object_type,
                object_value,
                field,
                field_definition,
                inner_type.as_ref(),
                argument_values,
            ),

            s::Type::NamedType(ref name) => self.resolve_field_value_for_named_type(
                object_value,
                field,
                field_definition,
                name,
                argument_values,
            ),

            s::Type::ListType(inner_type) => self.resolve_field_value_for_list_type(
                object_type,
                object_value,
                field,
                field_definition,
                inner_type.as_ref(),
                argument_values,
            ),
        }
    }

    /// Resolves the value of a field that corresponds to a named type.
    fn resolve_field_value_for_named_type(
        &mut self,
        object_value: &Option<q::Value>,
        field: &q::Field,
        field_definition: &s::Field,
        type_name: &s::Name,
        argument_values: &HashMap<&q::Name, q::Value>,
    ) -> Result<q::Value, ExecutionError> {
        // Try to resolve the type name into the actual type
        let named_type = sast::get_named_type(
            if self.introspecting {
                self.introspection_schema
            } else {
                &self.schema
            },
            type_name,
        ).ok_or(ExecutionError::NamedTypeError(type_name.to_string()))?;

        match named_type {
            // Let the resolver decide how the field (with the given object type)
            // is resolved into an entity based on the (potential) parent object
            s::TypeDefinition::Object(t) => if self.introspecting {
                Ok(self.introspection_resolver.resolve_object(
                    object_value,
                    &field.name,
                    field_definition,
                    t,
                    argument_values,
                ))
            } else {
                Ok(self.resolver.resolve_object(
                    object_value,
                    &field.name,
                    field_definition,
                    t,
                    argument_values,
                ))
            },

            // Let the resolver decide how values in the resolved object value
            // map to values of GraphQL enums
            s::TypeDefinition::Enum(t) => match object_value {
                Some(q::Value::Object(o)) => if self.introspecting {
                    Ok(self
                        .introspection_resolver
                        .resolve_enum_value(t, o.get(&field.name)))
                } else {
                    Ok(self.resolver.resolve_enum_value(t, o.get(&field.name)))
                },
                _ => Ok(q::Value::Null),
            },

            // Let the resolver decide how values in the resolved object value
            // map to values of GraphQL scalars
            s::TypeDefinition::Scalar(t) => match object_value {
                Some(q::Value::Object(o)) => if self.introspecting {
                    Ok(self
                        .introspection_resolver
                        .resolve_scalar_value(t, o.get(&field.name)))
                } else {
                    Ok(self.resolver.resolve_scalar_value(t, o.get(&field.name)))
                },
                _ => Ok(q::Value::Null),
            },

            // We will implement these later
            s::TypeDefinition::Interface(_) => unimplemented!(),
            s::TypeDefinition::Union(_) => unimplemented!(),

            _ => unimplemented!(),
        }
    }

    /// Resolves the value of a field that corresponds to a list type.
    fn resolve_field_value_for_list_type(
        &mut self,
        object_type: &s::ObjectType,
        object_value: &Option<q::Value>,
        field: &q::Field,
        field_definition: &s::Field,
        inner_type: &s::Type,
        argument_values: &HashMap<&q::Name, q::Value>,
    ) -> Result<q::Value, ExecutionError> {
        match inner_type {
            s::Type::NonNullType(inner_type) => self.resolve_field_value_for_list_type(
                object_type,
                object_value,
                field,
                field_definition,
                inner_type,
                argument_values,
            ),

            s::Type::NamedType(ref type_name) => {
                let named_type = sast::get_named_type(
                    if self.introspecting {
                        self.introspection_schema
                    } else {
                        &self.schema
                    },
                    type_name,
                ).expect("Failed to resolve named type inside list type");

                match named_type {
                    // Let the resolver decide how the list field (with the given item object type)
                    // is resolved into a entities based on the (potential) parent object
                    s::TypeDefinition::Object(t) => if self.introspecting {
                        Ok(self.introspection_resolver.resolve_objects(
                            object_value,
                            &field.name,
                            field_definition,
                            t,
                            argument_values,
                        ))
                    } else {
                        Ok(self.resolver.resolve_objects(
                            object_value,
                            &field.name,
                            field_definition,
                            t,
                            argument_values,
                        ))
                    },

                    // Let the resolver decide how values in the resolved object value
                    // map to values of GraphQL enums
                    s::TypeDefinition::Enum(t) => match object_value {
                        Some(q::Value::Object(o)) => if self.introspecting {
                            Ok(self
                                .introspection_resolver
                                .resolve_enum_values(t, o.get(&field.name)))
                        } else {
                            Ok(self.resolver.resolve_enum_values(t, o.get(&field.name)))
                        },
                        _ => Ok(q::Value::Null),
                    },

                    // Let the resolver decide how values in the resolved object value
                    // map to values of GraphQL scalars
                    s::TypeDefinition::Scalar(t) => match object_value {
                        Some(q::Value::Object(o)) => if self.introspecting {
                            Ok(self
                                .introspection_resolver
                                .resolve_scalar_values(t, o.get(&field.name)))
                        } else {
                            Ok(self.resolver.resolve_scalar_values(t, o.get(&field.name)))
                        },
                        _ => Ok(q::Value::Null),
                    },

                    // We will implement these later
                    s::TypeDefinition::Interface(_) => unimplemented!(),
                    s::TypeDefinition::Union(_) => unimplemented!(),

                    _ => unimplemented!(),
                }
            }

            // We don't support nested lists yet
            s::Type::ListType(_) => unimplemented!(),
        }
    }

    /// Ensures that a value matches the expected return type.
    fn complete_value(
        &mut self,
        field: &q::Field,
        field_type: &'a s::Type,
        fields: &Vec<q::Field>,
        resolved_value: q::Value,
    ) -> Result<q::Value, ExecutionError> {
        // Fail if the field type is non-null but the value is null
        if let s::Type::NonNullType(inner_type) = field_type {
            return match self.complete_value(field, inner_type, fields, resolved_value)? {
                q::Value::Null => Err(ExecutionError::NonNullError(
                    Position::from(field.position),
                    field.name.to_string(),
                )),
                v => Ok(v),
            };
        };

        // If the resolved value is null, return null
        if resolved_value == q::Value::Null {
            return Ok(resolved_value);
        }

        // Complete list values
        if let s::Type::ListType(inner_type) = field_type {
            return match resolved_value {
                // Complete list values individually
                q::Value::List(values) => {
                    let mut out = Vec::with_capacity(values.len());
                    for value in values.into_iter() {
                        out.push(self.complete_value(field, inner_type, fields, value)?);
                    }
                    Ok(q::Value::List(out))
                }

                // Return field error if the resolved value for the list is not a list
                _ => Err(ExecutionError::ListValueError(
                    Position::from(field.position),
                    field.name.to_string(),
                )),
            };
        }

        let named_type = if let s::Type::NamedType(name) = field_type {
            Some(
                sast::get_named_type(
                    if self.introspecting {
                        self.introspection_schema
                    } else {
                        &self.schema
                    },
                    name,
                ).unwrap(),
            )
        } else {
            None
        };

        match named_type {
            // Complete scalar values; we're assuming that the resolver has
            // already returned a valid value for the scalar type
            Some(s::TypeDefinition::Scalar(_)) => Ok(resolved_value),

            // Complete enum values; we're assuming that the resolver has
            // already returned a valid value for the enum type
            Some(s::TypeDefinition::Enum(_)) => Ok(resolved_value),

            // Complete object types recursively
            Some(s::TypeDefinition::Object(object_type)) => Ok(self.execute_selection_set(
                &Self::merge_selection_sets(fields),
                object_type,
                &Some(resolved_value),
            )),

            // Resolve interface and union types using the resolved value and complete
            // the value recursively
            Some(s::TypeDefinition::Interface(_)) | Some(s::TypeDefinition::Union(_)) => {
                let object_type = self.resolve_abstract_type(named_type.unwrap(), &resolved_value)?;
                Ok(self.execute_selection_set(
                    &Self::merge_selection_sets(fields),
                    object_type,
                    &Some(resolved_value),
                ))
            }

            _ => unimplemented!(),
        }
    }

    /// Resolves an abstract type (interface, union) into an object type based on the given value.
    fn resolve_abstract_type(
        &mut self,
        abstract_type: &'a s::TypeDefinition,
        object_value: &q::Value,
    ) -> Result<&'a s::ObjectType, ExecutionError> {
        // Let the resolver handle the type resolution, return an error if the resolution
        // yields nothing
        self.resolver
            .resolve_abstract_type(
                if self.introspecting {
                    self.introspection_schema
                } else {
                    &self.schema
                },
                abstract_type,
                object_value,
            )
            .ok_or(ExecutionError::AbstractTypeError(
                sast::get_type_name(abstract_type).to_string(),
            ))
    }

    /// Merges the selection sets of several fields into a single selection set.
    fn merge_selection_sets(fields: &Vec<q::Field>) -> q::SelectionSet {
        let (span, items) = fields
            .iter()
            .fold((None, vec![]), |(span, mut items), field| {
                (
                    // The overal span is the min/max spans of all merged selection sets
                    match span {
                        None => Some(field.selection_set.span.clone()),
                        Some((start, end)) => Some((
                            cmp::min(start, field.selection_set.span.0),
                            cmp::max(end, field.selection_set.span.1),
                        )),
                    },
                    // The overall selection is the result of merging the selections of all fields
                    {
                        items.extend_from_slice(field.selection_set.items.as_slice());
                        items
                    },
                )
            });

        q::SelectionSet {
            span: span.unwrap(),
            items,
        }
    }

    /// Coerces argument values into GraphQL values.
    fn coerce_argument_values(
        &mut self,
        object_type: &'a s::ObjectType,
        field: &q::Field,
    ) -> Result<HashMap<&'a q::Name, q::Value>, ExecutionError>
    where
        R1: Resolver,
        R2: Resolver,
    {
        let mut coerced_values = HashMap::new();

        if let Some(argument_definitions) = sast::get_argument_definitions(object_type, &field.name)
        {
            for argument_def in argument_definitions.iter() {
                match qast::get_argument_value(&field.arguments, &argument_def.name) {
                    // We don't support variables yet
                    Some(q::Value::Variable(_)) => unimplemented!(),

                    // There is no value, either use the default or fail
                    None => {
                        if let Some(ref default_value) = argument_def.default_value {
                            coerced_values.insert(&argument_def.name, default_value.clone());
                        } else if let s::Type::NonNullType(_) = argument_def.value_type {
                            return Err(ExecutionError::MissingArgumentError(
                                Position::from(field.position),
                                argument_def.name.to_owned(),
                            ));
                        };
                    }

                    // There is a value for the argument, attempt to coerce it to the
                    // value type of the argument definition
                    Some(v) => {
                        coerced_values.insert(
                            &argument_def.name,
                            self.coerce_argument_value(field, argument_def, v)?,
                        );
                    }
                };
            }
        };

        Ok(coerced_values)
    }

    /// Coerces a single argument value into a GraphQL value.
    fn coerce_argument_value(
        &mut self,
        field: &q::Field,
        argument: &s::InputValue,
        value: &q::Value,
    ) -> Result<q::Value, ExecutionError> {
        use graphql_parser::schema::Name;
        use values::coercion::coerce_value;

        let resolver = |name: &Name| {
            sast::get_named_type(
                if self.introspecting {
                    self.introspection_schema
                } else {
                    &self.schema
                },
                name,
            )
        };

        coerce_value(&value, &argument.value_type, &resolver).ok_or_else(|| {
            ExecutionError::InvalidArgumentError(
                Position::from(field.position),
                argument.name.to_owned(),
                value.clone(),
            )
        })
    }

    fn get_field_type(
        &mut self,
        object_type: &'a s::ObjectType,
        name: &s::Name,
    ) -> Option<(&'a s::Field, bool)> {
        // Resolve __schema and __Type using the introspection schema
        if Some(object_type) == sast::get_root_query_type(&self.schema) {
            if let Some(ref object_type) = sast::get_root_query_type(self.introspection_schema) {
                match sast::get_field_type(object_type, name).map(|t| (t, true)) {
                    Some(v) => return Some(v),
                    None => (),
                }
            }
        }

        sast::get_field_type(object_type, name).map(|t| (t, self.introspecting))
    }
}
