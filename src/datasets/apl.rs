pub struct Empty;
pub struct Populated {
    dataset_name: String,
    actions: Vec<AplAction>,
}

enum AplAction {
    Where {
        field: String,
        op: String,
        value: String,
    },
    Count,
    Project {
        fields: Vec<String>,
    },
    Summarize {
        aggregation: String,
        by: String,
    },
}

#[derive(Debug)]
pub struct AplBuilder<S> {
    state: S,
}

pub fn builder() -> AplBuilder<Empty> {
    AplBuilder { state: Empty }
}

impl AplBuilder<Empty> {
    pub fn dataset<S>(self, dataset_name: S) -> AplBuilder<Populated>
    where
        S: Into<String>,
    {
        AplBuilder {
            state: Populated {
                dataset_name: dataset_name.into(),
                actions: vec![],
            },
        }
    }
}

impl AplBuilder<Populated> {
    pub fn where_<F, O, V>(mut self, field: F, op: O, value: V) -> AplBuilder<Populated>
    where
        F: Into<String>,
        O: Into<String>,
        V: Into<String>,
    {
        self.state.actions.push(AplAction::Where {
            field: field.into(),
            op: op.into(),
            value: value.into(),
        });
        self
    }

    pub fn count(mut self) -> AplBuilder<Populated> {
        self.state.actions.push(AplAction::Count);
        self
    }

    pub fn project<S>(mut self, fields: Vec<S>) -> AplBuilder<Populated>
    where
        S: Into<String>,
    {
        let fields = fields.into_iter().map(|f| f.into()).collect();
        self.state.actions.push(AplAction::Project { fields });
        self
    }

    pub fn summarize<A, B>(mut self, aggregation: A, by: B) -> AplBuilder<Populated>
    where
        A: Into<String>,
        B: Into<String>,
    {
        self.state.actions.push(AplAction::Summarize {
            aggregation: aggregation.into(),
            by: by.into(),
        });
        self
    }

    pub fn build(self) -> String {
        let mut apl = format!("['{}']", self.state.dataset_name);

        for action in self.state.actions {
            match action {
                AplAction::Where { field, op, value } => {
                    apl.push_str(&format!(r#" | where {} {} "{}""#, field, op, value));
                }
                AplAction::Count => {
                    apl.push_str(" | count");
                }
                AplAction::Project { fields } => {
                    apl.push_str(&format!(" | project {}", fields.join(", ")));
                }
                AplAction::Summarize { aggregation, by } => {
                    apl.push_str(&format!(" | summarize {} by {}", aggregation, by));
                }
            }
        }

        apl
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_simple() {
        let apl = builder().dataset("foo").build();
        assert_eq!("['foo']", apl);
    }

    #[test]
    fn test_builder_advanced() {
        let apl = builder()
            .dataset("foo")
            .where_("foo", "==", "bar")
            .count()
            .project(vec!["foo"])
            .summarize("count()", "bin_auto(_time)")
            .build();
        assert_eq!(
            r#"['foo'] | where foo == "bar" | count | project foo | summarize count() by bin_auto(_time)"#,
            apl
        );
    }
}
