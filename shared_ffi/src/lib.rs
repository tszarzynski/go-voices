//! The `BoltFFI` interface to the core, for the Swift and Kotlin shells.

use crux_core::{
    Core, ResolveError,
    bridge::{Bridge, BridgeError, EffectId},
};

use shared::GoVoices;

/// The core, as the Swift and Kotlin shells call it: bytes in, bytes out.
pub struct CoreFfi {
    core: Bridge<GoVoices>,
}

impl Default for CoreFfi {
    fn default() -> Self {
        Self::new()
    }
}

#[boltffi::export]
impl CoreFfi {
    /// A fresh core.
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: Bridge::new(Core::new()),
        }
    }

    /// Sends an event to the app; returns the effect requests.
    ///
    /// # Panics
    ///
    /// If the event isn't a valid `Event` (a shell bug).
    #[must_use]
    pub fn update(&self, data: &[u8]) -> Vec<u8> {
        let mut effects = Vec::new();
        self.core
            .update(data, &mut effects)
            .unwrap_or_else(|e| panic!("invalid event: {e}"));
        effects
    }

    /// Resolves an effect; returns the effects it requests.
    ///
    /// An item for a stream the core has finished with (a scan result just
    /// after the scan stopped, say) is ignored: shells deliver stream items
    /// asynchronously, so one can always arrive late.
    ///
    /// # Panics
    ///
    /// If the output isn't valid for the request (a shell bug), or the id
    /// was never issued.
    #[must_use]
    pub fn resolve(&self, id: u32, data: &[u8]) -> Vec<u8> {
        let mut effects = Vec::new();
        match self.core.resolve(EffectId(id), data, &mut effects) {
            Ok(()) => effects,
            Err(BridgeError::ProcessResponse(
                ResolveError::NotFound(_) | ResolveError::FinishedMany,
            )) => Vec::new(),
            Err(e) => panic!("invalid output for request {id}: {e}"),
        }
    }

    /// The current `ViewModel`.
    ///
    /// # Panics
    ///
    /// If the view model can't be serialized (a core bug).
    #[must_use]
    pub fn view(&self) -> Vec<u8> {
        let mut view_model = Vec::new();
        self.core
            .view(&mut view_model)
            .unwrap_or_else(|e| panic!("unserializable view model: {e}"));
        view_model
    }
}
