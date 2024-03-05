use eframe::{egui::{self, Button, RadioButton, TextEdit, Slider}, emath::Align2};
use egui_toast::{Toasts, Toast, ToastKind, ToastOptions};
use std::{
    process::Command, sync::mpsc::{Receiver, Sender}, path::{Path, PathBuf}, 
};

use crate::{*, popup::{MessageLog, LogKind}};
