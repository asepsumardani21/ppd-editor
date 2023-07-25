use std::{path::PathBuf, thread};

use anyhow::{anyhow, Result};
use eframe::{egui::Context, epaint::Pos2, Frame};
use paperdoll_tar::{
    paperdoll::{
        factory::PaperdollFactory,
        image::{ColorType, ImageData},
    },
    EXTENSION_NAME,
};

use crate::{
    adapter::{DollAdapter, FragmentAdapter, FragmentFilter, ImageAdapter, SlotAdapter},
    common::{upload_image_to_texture, upload_ppd_textures, TextureData},
    fs::{create_file, open_image_rgba, select_file, select_texture},
    viewer,
};

use super::{DialogOption, EditorApp, APP_TITLE};

pub enum Action {
    AppQuit,
    AppTitleChanged(Option<String>),
    CursorMoved(Option<Pos2>),
    DollCreate,
    DollAdapterBackgroundRemove,
    DollAdapterBackgroundUpload,
    DollBackgroundRemove(u32),
    DollBackgroundUpload(u32),
    DollEdit(u32),
    DollEditConfirm(Option<u32>),
    DollRemoveConfirm(u32),
    DollRemoveRequest(u32),
    DollResizeToBackground(u32),
    FileNew,
    FileOpen,
    FileSave,
    FileSaveAs,
    FragmentAdapterBackgroundUpload,
    FragmentBackgroundUpload(u32),
    FragmentCreate,
    FragmentEdit(u32),
    FragmentEditCancel(Option<u32>),
    FragmentEditConfirm(Option<u32>),
    FragmentRemoveConfirm(u32),
    FragmentRemoveRequest(u32),
    PpdLoad(PaperdollFactory),
    PpdChanged,
    // PpdPreview,
    SlotAdapterFragmentFilter,
    SlotCopy(u32),
    SlotCreate,
    SlotDuplicate(u32, u32),
    SlotEdit(u32),
    SlotEditCancel(Option<u32>),
    SlotEditConfirm(Option<u32>),
    SlotLower(u32, u32),
    SlotLowerBottom(u32, u32),
    SlotPaste(u32),
    SlotRaise(u32, u32),
    SlotRaiseTop(u32, u32),
    SlotRemoveConfirm(u32),
    SlotRemoveRequest(u32),
    WindowDollVisible(bool),
    WindowFragmentVisible(bool),
    WindowSlotVisible(bool),
}

impl EditorApp {
    pub(super) fn handle_actions(&mut self, ctx: &Context, frame: &mut Frame) -> Result<()> {
        while let Some(action) = self.actions.pop_front() {
            match action {
                Action::AppQuit => frame.close(),
                Action::AppTitleChanged(title) => {
                    let title =
                        format!("{} - {}", APP_TITLE, title.unwrap_or("Untitled".to_owned()));

                    frame.set_window_title(&title)
                }
                Action::CursorMoved(position) => {
                    self.cursor_position = position;
                }
                Action::DollAdapterBackgroundRemove => {
                    if let Some(adapter_doll) = self.adapter_doll.as_mut() {
                        adapter_doll.path = String::default();

                        adapter_doll.image = ImageAdapter::default();
                    }
                }
                Action::DollAdapterBackgroundUpload => {
                    if self.adapter_doll.is_none() {
                        continue;
                    }

                    if let Some((path, texture, pixels)) = self.upload_texture("doll", ctx) {
                        if let Some(adapter_doll) = self.adapter_doll.as_mut() {
                            adapter_doll.path = path.to_string_lossy().to_string();

                            adapter_doll.image.width = texture.width;
                            adapter_doll.image.height = texture.height;
                            adapter_doll.image.pixels = pixels;
                            adapter_doll.image.texture = Some(texture.texture);
                        }
                    }
                }
                Action::DollBackgroundRemove(id) => {
                    if let Some(doll) = self.ppd.get_doll_mut(id) {
                        self.textures_doll.remove(&id);

                        doll.path = String::default();

                        doll.image = ImageData::default();
                    }
                }
                Action::DollBackgroundUpload(id) => {
                    if self.ppd.get_doll(id).is_none() {
                        continue;
                    }

                    if let Some((path, texture, pixels)) = self.upload_texture("doll", ctx) {
                        if let Some(doll) = self.ppd.get_doll_mut(id) {
                            doll.path = path.to_string_lossy().to_string();

                            doll.image.width = texture.width;
                            doll.image.height = texture.height;
                            doll.image.pixels = pixels;

                            self.textures_doll.insert(id, texture);
                        }
                    }
                }
                Action::DollCreate => {
                    self.actived_doll = None;

                    self.adapter_doll = Some(DollAdapter::default());

                    self.actions.push_back(Action::WindowDollVisible(true));
                }
                Action::DollEdit(id) => {
                    if let Some(doll) = self.ppd.get_doll(id) {
                        self.actived_doll = Some(id);

                        self.adapter_doll = Some(doll.into());
                        if let Some(adapter_doll) = self.adapter_doll.as_mut() {
                            adapter_doll.image.texture = self
                                .textures_doll
                                .get(&id)
                                .map(|texture| texture.texture.clone());
                        }
                    }
                }
                Action::DollEditConfirm(id) => {
                    let id = id.or_else(|| self.ppd.add_doll().ok());

                    self.actived_doll = id;

                    if let Some(id) = id {
                        if let Some(doll) = self.ppd.get_doll_mut(id) {
                            if let Some(adapter_doll) = self.adapter_doll.take() {
                                doll.desc = adapter_doll.desc;
                                doll.path = adapter_doll.path;
                                doll.width = adapter_doll.width;
                                doll.height = adapter_doll.height;

                                if let Some(texture) = adapter_doll.image.texture {
                                    self.textures_doll.insert(
                                        id,
                                        TextureData {
                                            width: adapter_doll.image.width,
                                            height: adapter_doll.image.height,
                                            texture,
                                        },
                                    );

                                    doll.image.width = adapter_doll.image.width;
                                    doll.image.height = adapter_doll.image.height;
                                    doll.image.color_type = ColorType::Rgba;
                                    doll.image.pixels = adapter_doll.image.pixels;
                                }
                            }
                        }
                    }

                    self.window_doll_error = None;
                }
                Action::DollRemoveConfirm(id) => {
                    self.actived_doll = None;

                    self.ppd.remove_doll(id);
                }
                Action::DollRemoveRequest(id) => {
                    self.dialog_visible = true;

                    self.dialog_option =
                        DialogOption::confirm(&format!("Really delete doll {}?", id))
                            .primary_action(Action::DollRemoveConfirm(id));
                }
                Action::DollResizeToBackground(id) => {
                    if let Some(doll) = self.ppd.get_doll_mut(id) {
                        if !doll.image.is_empty() {
                            doll.width = doll.image.width;
                            doll.height = doll.image.height;
                        }
                    }
                }
                Action::FileNew => {
                    self.actions
                        .push_back(Action::PpdLoad(PaperdollFactory::default()));

                    self.actions.push_back(Action::AppTitleChanged(None));

                    self.config.file_path = None;
                }
                Action::FileOpen => {
                    if let Some(path) = select_file() {
                        self.actions
                            .push_back(Action::PpdLoad(paperdoll_tar::load(&path)?));

                        self.actions.push_back(Action::AppTitleChanged(Some(
                            path.to_string_lossy().to_string(),
                        )));

                        self.config.file_path = Some(path);
                    }
                }
                Action::FileSave => {
                    let path = if let Some(path) = &self.config.file_path {
                        Some(path.to_owned())
                    } else {
                        let name = (!self.ppd.meta.name.is_empty())
                            .then_some(self.ppd.meta.name.as_str())
                            .unwrap_or("Untitled");

                        let filename = format!("{}.{}", name.replace(" ", "_"), EXTENSION_NAME);

                        create_file(&filename)
                    };

                    self.file_save_to_path(path)?;
                }
                Action::FileSaveAs => {
                    let name = (!self.ppd.meta.name.is_empty())
                        .then_some(self.ppd.meta.name.as_str())
                        .unwrap_or("Untitled");

                    let filename = format!("{}.{}", name.replace(" ", "_"), EXTENSION_NAME);

                    let path = create_file(&filename);

                    self.file_save_to_path(path)?;
                }
                Action::FragmentAdapterBackgroundUpload => {
                    if self.adapter_fragment.is_none() {
                        continue;
                    }

                    if let Some((path, texture, pixels)) = self.upload_texture("fragment", ctx) {
                        if let Some(adapter_fragment) = self.adapter_fragment.as_mut() {
                            adapter_fragment.path = path.to_string_lossy().to_string();

                            adapter_fragment.image.width = texture.width;
                            adapter_fragment.image.height = texture.height;
                            adapter_fragment.image.pixels = pixels;
                            adapter_fragment.image.texture = Some(texture.texture);
                        }
                    }
                }
                Action::FragmentBackgroundUpload(id) => {
                    if self.ppd.get_fragment(id).is_none() {
                        continue;
                    }

                    if let Some((path, texture, pixels)) = self.upload_texture("fragment", ctx) {
                        if let Some(fragment) = self.ppd.get_fragment_mut(id) {
                            fragment.path = path.to_string_lossy().to_string();

                            self.textures_fragment.insert(
                                id,
                                TextureData {
                                    width: texture.width,
                                    height: texture.height,
                                    texture: texture.texture,
                                },
                            );

                            fragment.image.width = texture.width;
                            fragment.image.height = texture.height;
                            fragment.image.color_type = ColorType::Rgba;
                            fragment.image.pixels = pixels;
                        }
                    }
                }
                Action::FragmentCreate => {
                    self.actived_fragment = None;

                    self.adapter_fragment = Some(FragmentAdapter::default());

                    self.actions.push_back(Action::WindowFragmentVisible(true));
                }
                Action::FragmentEdit(id) => {
                    if let Some(fragment) = self.ppd.get_fragment(id) {
                        self.actived_fragment = Some(id);

                        self.adapter_fragment = Some(fragment.into());
                        if let Some(adapter_fragment) = self.adapter_fragment.as_mut() {
                            adapter_fragment.image.texture = self
                                .textures_fragment
                                .get(&id)
                                .map(|texture| texture.texture.clone());
                        }

                        self.actions.push_back(Action::WindowFragmentVisible(true));
                    }
                }
                Action::FragmentEditCancel(id) => {
                    self.window_fragment_error = None;

                    if id.is_none() || self.actived_fragment.is_none() {
                        continue;
                    }

                    if let Some(id) = id {
                        if let Some(fragment) = self.ppd.get_fragment_mut(id) {
                            if let Some(adapter_fragment) = self.adapter_fragment.take() {
                                fragment.desc = adapter_fragment.desc;
                                fragment.pivot = adapter_fragment.pivot;
                                fragment.path = adapter_fragment.path;

                                fragment.image.width = adapter_fragment.image.width;
                                fragment.image.height = adapter_fragment.image.height;
                                fragment.image.color_type = ColorType::Rgba;
                                fragment.image.pixels = adapter_fragment.image.pixels;

                                if let Some(texture) = adapter_fragment.image.texture {
                                    self.textures_fragment.insert(
                                        id,
                                        TextureData {
                                            width: adapter_fragment.image.width,
                                            height: adapter_fragment.image.height,
                                            texture: texture.clone(),
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
                Action::FragmentEditConfirm(id) => {
                    let is_create_mode = id.is_none();

                    if is_create_mode {
                        if let Some(adapter_fragment) = &self.adapter_fragment {
                            if adapter_fragment.path.is_empty() {
                                self.window_fragment_error = Some("Image is required".to_owned());

                                bail!("Fragment requires an image.");
                            }
                        }
                    }

                    let id = id.or_else(|| self.ppd.add_fragment().ok());

                    self.actived_fragment = id;

                    if is_create_mode {
                        if let Some(id) = id {
                            if let Some(fragment) = self.ppd.get_fragment_mut(id) {
                                if let Some(adapter_fragment) = self.adapter_fragment.take() {
                                    fragment.desc = adapter_fragment.desc;
                                    fragment.pivot = adapter_fragment.pivot;
                                    fragment.path = adapter_fragment.path;

                                    if let Some(texture) = adapter_fragment.image.texture {
                                        self.textures_fragment.insert(
                                            id,
                                            TextureData {
                                                width: adapter_fragment.image.width,
                                                height: adapter_fragment.image.height,
                                                texture,
                                            },
                                        );

                                        fragment.image.width = adapter_fragment.image.width;
                                        fragment.image.height = adapter_fragment.image.height;
                                        fragment.image.color_type = ColorType::Rgba;
                                        fragment.image.pixels = adapter_fragment.image.pixels;
                                    }
                                }
                            }
                        }
                    }

                    self.window_fragment_error = None;
                }
                Action::FragmentRemoveConfirm(id) => {
                    self.actived_fragment = None;

                    self.ppd.remove_fragment(id);
                }
                Action::FragmentRemoveRequest(id) => {
                    self.dialog_visible = true;

                    self.dialog_option =
                        DialogOption::confirm(&format!("Really delete fragment {}?", id))
                            .primary_action(Action::FragmentRemoveConfirm(id));
                }
                Action::PpdLoad(ppd) => {
                    self.ppd = ppd;

                    self.actions.push_back(Action::PpdChanged);
                }
                Action::PpdChanged => {
                    let ppd = &self.ppd;

                    self.visible_slots = ppd.slots().map(|(id, _)| *id).collect();

                    self.adapter_doll = None;
                    self.adapter_fragment = None;
                    self.adapter_slot = None;

                    self.actived_doll = ppd.dolls().nth(0).and_then(|(id, _)| Some(*id));
                    self.actived_fragment = None;
                    self.adapter_slot = None;

                    let (textures_doll, textures_fragment) = upload_ppd_textures(ppd, ctx);

                    self.textures_doll = textures_doll;
                    self.textures_fragment = textures_fragment;

                    self.actions.push_back(Action::WindowDollVisible(false));
                    self.actions.push_back(Action::WindowFragmentVisible(false));
                    self.actions.push_back(Action::WindowSlotVisible(false));
                }
                // Action::PpdPreview => {
                //     let manifest = self.ppd.to_manifest();

                //     let ppd = PaperdollFactory::from_manifest(manifest)?;

                //     thread::spawn(move || {
                //         let native_options = eframe::NativeOptions::default();

                //         eframe::run_native(
                //             viewer::APP_TITLE,
                //             native_options,
                //             Box::new(|cc| viewer::setup_eframe(cc, Some(ppd))),
                //         )
                //         .map_err(|e| anyhow!("EEE: {}", e));
                //     });
                // }
                Action::SlotAdapterFragmentFilter => {
                    self.filter_slot_fragment();
                }
                Action::SlotCopy(id) => {
                    self.slot_copy = Some(id);
                }
                Action::SlotCreate => {
                    self.actived_slot = None;

                    self.adapter_slot = Some(SlotAdapter::default());

                    self.filter_slot_fragment();

                    self.actions.push_back(Action::WindowSlotVisible(true));
                }
                Action::SlotDuplicate(doll_id, slot_id) => {
                    self.actions.push_back(Action::SlotCopy(slot_id));

                    self.actions.push_back(Action::SlotPaste(doll_id));
                }
                Action::SlotEdit(id) => {
                    if let Some(slot) = self.ppd.get_slot(id) {
                        self.actived_slot = Some(id);

                        self.adapter_slot = Some(slot.into());

                        self.filter_slot_fragment();

                        self.actions.push_back(Action::WindowSlotVisible(true));
                    }
                }
                Action::SlotEditCancel(id) => {
                    self.window_slot_error = None;

                    if id.is_none() || self.adapter_slot.is_none() {
                        continue;
                    }

                    if let Some(id) = id {
                        if let Some(slot) = self.ppd.get_slot_mut(id) {
                            if let Some(adapter_slot) = self.adapter_slot.take() {
                                slot.desc = adapter_slot.desc;
                                slot.required = adapter_slot.required;
                                slot.constrainted = adapter_slot.constrainted;
                                slot.position = adapter_slot.position;
                                slot.width = adapter_slot.width;
                                slot.height = adapter_slot.height;
                                slot.anchor = adapter_slot.anchor;
                                slot.candidates = adapter_slot.candidates;
                            }
                        }
                    }
                }
                Action::SlotEditConfirm(id) => {
                    let is_create_mode = id.is_none();

                    let id = id.or_else(|| self.ppd.add_slot().ok());

                    self.actived_slot = id;

                    if is_create_mode {
                        if let Some(id) = id {
                            if let Some(slot) = self.ppd.get_slot_mut(id) {
                                if let Some(adapter_slot) = self.adapter_slot.take() {
                                    slot.desc = adapter_slot.desc;
                                    slot.required = adapter_slot.required;
                                    slot.constrainted = adapter_slot.constrainted;
                                    slot.position = adapter_slot.position;
                                    slot.width = adapter_slot.width;
                                    slot.height = adapter_slot.height;
                                    slot.anchor = adapter_slot.anchor;
                                    slot.candidates = adapter_slot.candidates;
                                }
                            }

                            if let Some(doll_id) = self.actived_doll {
                                if let Some(doll) = self.ppd.get_doll_mut(doll_id) {
                                    doll.slots.push(id);
                                }
                            }

                            self.visible_slots.insert(id);
                        }
                    }

                    self.window_slot_error = None;
                }
                Action::SlotLower(doll_id, slot_id) => {
                    if let Some(doll) = self.ppd.get_doll_mut(doll_id) {
                        if let Some(position) = doll
                            .slots
                            .iter()
                            .position(|v| *v == slot_id)
                            .and_then(|index| (index < doll.slots.len() - 1).then_some(index))
                        {
                            doll.slots.swap(position, position + 1);
                        }
                    }
                }
                Action::SlotLowerBottom(doll_id, slot_id) => {
                    if let Some(doll) = self.ppd.get_doll_mut(doll_id) {
                        let len = doll.slots.len();

                        if let Some(position) = doll
                            .slots
                            .iter()
                            .position(|v| *v == slot_id)
                            .and_then(|index| (index < len - 1).then_some(index))
                        {
                            let id = doll.slots.remove(position);

                            doll.slots.push(id);
                        }
                    }
                }
                Action::SlotPaste(doll_id) => {
                    if self.slot_copy.is_none() {
                        continue;
                    }

                    let slot_copy = self.slot_copy.unwrap();

                    if self.ppd.get_doll(doll_id).is_none()
                        || self.ppd.get_slot(slot_copy).is_none()
                    {
                        continue;
                    }

                    let id = self.ppd.add_slot()?;

                    self.actived_slot = Some(id);

                    let slot_copy = self.ppd.get_slot(slot_copy).unwrap();
                    let desc = slot_copy.desc.clone();
                    let required = slot_copy.required;
                    let constrainted = slot_copy.constrainted;
                    let position = slot_copy.position;
                    let width = slot_copy.width;
                    let height = slot_copy.height;
                    let anchor = slot_copy.anchor;
                    let candidates = slot_copy.candidates.clone();

                    if let Some(slot) = self.ppd.get_slot_mut(id) {
                        slot.desc = desc.clone();
                        slot.required = required;
                        slot.constrainted = constrainted;
                        slot.position = position;
                        slot.width = width;
                        slot.height = height;
                        slot.anchor = anchor;
                        slot.candidates = candidates;

                        if let Some(doll) = self.ppd.get_doll_mut(doll_id) {
                            doll.slots.push(id);
                        }

                        self.visible_slots.insert(id);

                        self.slot_copy = None;
                    }
                }
                Action::SlotRaise(doll_id, slot_id) => {
                    if let Some(doll) = self.ppd.get_doll_mut(doll_id) {
                        if let Some(position) = doll
                            .slots
                            .iter()
                            .position(|v| *v == slot_id)
                            .and_then(|index| (index > 0).then_some(index))
                        {
                            doll.slots.swap(position, position - 1);
                        }
                    }
                }
                Action::SlotRaiseTop(doll_id, slot_id) => {
                    if let Some(doll) = self.ppd.get_doll_mut(doll_id) {
                        if let Some(position) = doll
                            .slots
                            .iter()
                            .position(|v| *v == slot_id)
                            .and_then(|index| (index > 0).then_some(index))
                        {
                            let id = doll.slots.remove(position);

                            doll.slots.insert(0, id);
                        }
                    }
                }
                Action::SlotRemoveConfirm(id) => {
                    self.actived_slot = None;

                    self.ppd.remove_slot(id);

                    self.visible_slots.remove(&id);
                }
                Action::SlotRemoveRequest(id) => {
                    self.dialog_visible = true;

                    self.dialog_option =
                        DialogOption::confirm(&format!("Really delete slot {}?", id))
                            .primary_action(Action::SlotRemoveConfirm(id));
                }
                Action::WindowDollVisible(visible) => {
                    if !visible && self.window_doll_error.is_some() {
                        continue;
                    }

                    self.window_doll_visible = visible;

                    if visible {
                        self.window_doll_error = None;
                    }
                }
                Action::WindowFragmentVisible(visible) => {
                    if !visible && self.window_fragment_error.is_some() {
                        continue;
                    }

                    self.window_fragment_visible = visible;

                    if visible {
                        self.window_fragment_error = None;
                    }
                }
                Action::WindowSlotVisible(visible) => {
                    if !visible && self.window_slot_error.is_some() {
                        continue;
                    }

                    self.window_slot_visible = visible;

                    if visible {
                        self.window_slot_error = None;
                    }
                }
            }
        }

        Ok(())
    }

    fn file_save_to_path(&mut self, path: Option<PathBuf>) -> Result<()> {
        if let Some(path) = &path {
            paperdoll_tar::save(&self.ppd.to_manifest(), path)?;
        }

        self.actions.push_back(Action::AppTitleChanged(
            path.as_ref()
                .and_then(|path| Some(path.to_string_lossy().to_string())),
        ));

        self.config.file_path = path;

        Ok(())
    }

    fn filter_slot_fragment(&mut self) {
        if let Some(ref mut adapter_slot) = &mut self.adapter_slot {
            adapter_slot.actived_fragments = match adapter_slot.fragments_filter {
                FragmentFilter::All => self.ppd.fragments().map(|(id, _)| *id).collect(),
                FragmentFilter::IsCandidate => self
                    .ppd
                    .fragments()
                    .filter(|(id, _)| adapter_slot.candidates.contains(&id))
                    .map(|(id, _)| *id)
                    .collect(),
                FragmentFilter::IsNotCandidate => self
                    .ppd
                    .fragments()
                    .filter(|(id, _)| !adapter_slot.candidates.contains(&id))
                    .map(|(id, _)| *id)
                    .collect(),
            };

            if !adapter_slot.fragments_filter_keyword.is_empty() {
                adapter_slot.actived_fragments = adapter_slot
                    .actived_fragments
                    .iter()
                    .filter(|id| {
                        self.ppd.get_fragment(**id).map_or(false, |fragment| {
                            fragment
                                .desc
                                .contains(&adapter_slot.fragments_filter_keyword)
                        })
                    })
                    .map(|v| *v)
                    .collect();
            }
        }
    }

    fn upload_texture(
        &mut self,
        name: impl Into<String>,
        ctx: &Context,
    ) -> Option<(PathBuf, TextureData, Vec<u8>)> {
        if let Some(path) = select_texture() {
            return match open_image_rgba(&path) {
                Ok(image) => {
                    let texture = upload_image_to_texture(&image, name, ctx);

                    Some((path, texture, image.pixels))
                }
                Err(err) => {
                    log::error!("Failed to open texture: '{:?}'. {}", path, err);

                    None
                }
            };
        }

        None
    }
}
