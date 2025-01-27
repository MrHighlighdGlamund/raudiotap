use std::sync::Arc;
use nih_plug::params::Params;
use nih_plug_egui::EguiState;



#[derive(Params)]
pub struct RaudiotapParams {
    pub editor_state: Arc<EguiState>,
}
impl Default for RaudiotapParams {
    fn default() -> Self {
        let (width, height) = (500, 300);

        Self {
            editor_state: EguiState::from_size(width, height),
        }
    }
}
