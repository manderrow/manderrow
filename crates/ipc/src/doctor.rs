use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::ControlFlow;

use uuid::Uuid;

use crate::{C2SMessage, DoctorFix, DoctorReport, S2CMessage};

pub struct PatientChoiceReceiver<T> {
    id: Uuid,
    _marker: PhantomData<T>,
}

impl<T: serde::Serialize> PatientChoiceReceiver<T> {
    pub fn new(
        translation_key: impl Into<String>,
        message: Option<String>,
        message_args: Option<HashMap<String, String>>,
        fixes: impl IntoIterator<Item = DoctorFix<T>>,
    ) -> (Self, C2SMessage) {
        let fixes = fixes
            .into_iter()
            .map(|fix| {
                let serde_json::Value::String(id) =
                    serde_json::to_value(fix.id).expect("Unable to serialize id")
                else {
                    panic!("Id must serialize to a string")
                };
                DoctorFix { id, ..fix }
            })
            .collect::<Vec<_>>();
        let translation_key = translation_key.into();
        let id = Uuid::new_v4();
        (
            Self {
                id,
                _marker: PhantomData,
            },
            C2SMessage::DoctorReport(DoctorReport {
                id,
                translation_key,
                message,
                message_args,
                fixes,
            }),
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PromptError {
    #[error("failed to decode patient choice received from server: {0}")]
    Decode(serde_json::Error),
}

impl<T: serde::de::DeserializeOwned> PatientChoiceReceiver<T> {
    pub fn process(self, response: S2CMessage) -> Result<ControlFlow<T, Self>, PromptError> {
        match response {
            S2CMessage::PatientResponse {
                id: resp_id,
                choice,
            } if resp_id == self.id => Ok(ControlFlow::Break(
                serde_json::from_value(serde_json::Value::String(choice))
                    .map_err(PromptError::Decode)?,
            )),
            _ => Ok(ControlFlow::Continue(self)),
        }
    }
}
