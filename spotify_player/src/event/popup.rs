use crate::command::Action;

use super::*;

/// handles a key sequence for a popup
pub fn handle_key_sequence_for_popup(
    key_sequence: &KeySequence,
    send: &mpsc::Sender<ClientRequest>,
    state: &SharedState,
    ui: &mut UIStateGuard,
) -> Result<bool> {
    match ui.popup.as_ref().unwrap() {
        PopupState::ContextSearch(_) => {
            handle_key_sequence_for_search_popup(key_sequence, send, state, ui)
        }
        PopupState::ArtistList(..) => handle_key_sequence_for_list_popup(
            key_sequence,
            state,
            ui,
            match ui.popup {
                Some(PopupState::ArtistList(ref artists, _)) => artists.len(),
                _ => unreachable!(),
            },
            |_, _| {},
            |ui: &mut UIStateGuard, id: usize| -> Result<()> {
                let artists = match ui.popup {
                    Some(PopupState::ArtistList(ref artists, _)) => artists,
                    _ => unreachable!(),
                };

                let context_id = ContextId::Artist(artists[id].id.clone());
                send.send(ClientRequest::GetContext(context_id.clone()))?;
                ui.new_page(PageState::Browsing(context_id));

                Ok(())
            },
            |ui: &mut UIStateGuard| {
                ui.popup = None;
            },
        ),
        PopupState::UserPlaylistList(action, playlists, _) => {
            match action {
                PlaylistPopupAction::Browse => {
                    let playlist_uris = playlists.iter().map(|p| p.id.uri()).collect::<Vec<_>>();

                    handle_key_sequence_for_context_browsing_list_popup(
                        key_sequence,
                        send,
                        state,
                        ui,
                        playlist_uris,
                        model::Type::Playlist,
                    )
                }
                PlaylistPopupAction::AddTrack(ref track_id) => {
                    let track_id = track_id.clone();

                    handle_key_sequence_for_list_popup(
                        key_sequence,
                        state,
                        ui,
                        {
                            match ui.popup {
                                Some(PopupState::UserPlaylistList(_, ref playlists, _)) => {
                                    playlists.len()
                                }
                                _ => unreachable!(),
                            }
                        },
                        |_, _| {},
                        |ui: &mut UIStateGuard, id: usize| -> Result<()> {
                            let playlists = match ui.popup {
                                Some(PopupState::UserPlaylistList(_, ref playlists, _)) => {
                                    playlists
                                }
                                _ => unreachable!(),
                            };

                            // when adding a new track to a playlist, we need to remove
                            // the cache for that playlist
                            state
                                .player
                                .write()
                                .unwrap()
                                .context_cache
                                .pop(&playlists[id].id.uri());

                            send.send(ClientRequest::AddTrackToPlaylist(
                                playlists[id].id.clone(),
                                track_id.clone(),
                            ))?;
                            ui.popup = None;
                            Ok(())
                        },
                        |ui: &mut UIStateGuard| {
                            ui.popup = None;
                        },
                    )
                }
            }
        }
        PopupState::UserFollowedArtistList(_) => {
            let player = state.player.read().unwrap();
            let artist_uris = player
                .user_followed_artists
                .iter()
                .map(|a| a.id.uri())
                .collect::<Vec<_>>();

            handle_key_sequence_for_context_browsing_list_popup(
                key_sequence,
                send,
                state,
                ui,
                artist_uris,
                model::Type::Artist,
            )
        }
        PopupState::UserSavedAlbumList(_) => {
            let player = state.player.read().unwrap();
            let album_uris = player
                .user_saved_albums
                .iter()
                .map(|a| a.id.uri())
                .collect::<Vec<_>>();

            handle_key_sequence_for_context_browsing_list_popup(
                key_sequence,
                send,
                state,
                ui,
                album_uris,
                model::Type::Album,
            )
        }
        PopupState::ThemeList(_, _) => handle_key_sequence_for_list_popup(
            key_sequence,
            state,
            ui,
            match ui.popup {
                Some(PopupState::ThemeList(ref themes, _)) => themes.len(),
                _ => unreachable!(),
            },
            |ui: &mut UIStateGuard, id: usize| {
                ui.theme = match ui.popup {
                    Some(PopupState::ThemeList(ref themes, _)) => themes[id].clone(),
                    _ => unreachable!(),
                };
            },
            |ui: &mut UIStateGuard, _| -> Result<()> {
                ui.popup = None;
                Ok(())
            },
            |ui: &mut UIStateGuard| {
                ui.theme = match ui.popup {
                    Some(PopupState::ThemeList(ref themes, _)) => themes[0].clone(),
                    _ => unreachable!(),
                };
                ui.popup = None;
            },
        ),
        PopupState::DeviceList(_) => {
            let player = state.player.read().unwrap();

            handle_key_sequence_for_list_popup(
                key_sequence,
                state,
                ui,
                player.devices.len(),
                |_, _| {},
                |ui: &mut UIStateGuard, id: usize| -> Result<()> {
                    send.send(ClientRequest::Player(PlayerRequest::TransferPlayback(
                        player.devices[id].id.clone(),
                        true,
                    )))?;
                    ui.popup = None;
                    Ok(())
                },
                |ui: &mut UIStateGuard| {
                    ui.popup = None;
                },
            )
        }
        PopupState::CommandHelp(_) => {
            handle_key_sequence_for_command_help_popup(key_sequence, state, ui)
        }
        PopupState::ActionList(ref item, _) => {
            handle_key_sequence_for_action_list_popup(item.actions(), key_sequence, send, state, ui)
        }
    }
}

/// handles a key sequence for a context search popup
fn handle_key_sequence_for_search_popup(
    key_sequence: &KeySequence,
    send: &mpsc::Sender<ClientRequest>,
    state: &SharedState,
    ui: &mut UIStateGuard,
) -> Result<bool> {
    let query = match ui.popup {
        Some(PopupState::ContextSearch(ref mut query)) => query,
        _ => unreachable!(),
    };
    if key_sequence.keys.len() == 1 {
        if let Key::None(c) = key_sequence.keys[0] {
            match c {
                KeyCode::Char(c) => {
                    query.push(c);
                    ui.window.select(Some(0));
                    return Ok(true);
                }
                KeyCode::Backspace => {
                    if !query.is_empty() {
                        query.pop().unwrap();
                        ui.window.select(Some(0));
                    }
                    return Ok(true);
                }
                _ => {}
            }
        }
    }

    let command = state
        .keymap_config
        .find_command_from_key_sequence(key_sequence);

    match command {
        Some(command) => match command {
            Command::ClosePopup => {
                ui.window.select(Some(0));
                ui.popup = None;
            }
            _ => match ui.current_page() {
                PageState::Recommendations(..) => {
                    return window::handle_key_sequence_for_recommendation_window(
                        key_sequence,
                        send,
                        state,
                        ui,
                    );
                }
                PageState::Browsing(_) | PageState::CurrentPlaying => {
                    return window::handle_key_sequence_for_context_window(
                        key_sequence,
                        send,
                        state,
                        ui,
                    );
                }
                _ => {}
            },
        },
        None => return Ok(false),
    }
    Ok(true)
}

/// handles a key sequence for a context list popup in which
/// each item represents a context
///
/// In addition to application's states and the key sequence,
/// the function requires to specify:
/// - `uris`: a list of context URIs
/// - `uri_type`: an enum represents the type of a context in the list (`playlist`, `artist`, etc)
fn handle_key_sequence_for_context_browsing_list_popup(
    key_sequence: &KeySequence,
    send: &mpsc::Sender<ClientRequest>,
    state: &SharedState,
    ui: &mut UIStateGuard,
    uris: Vec<String>,
    context_type: model::Type,
) -> Result<bool> {
    handle_key_sequence_for_list_popup(
        key_sequence,
        state,
        ui,
        uris.len(),
        |_, _| {},
        |ui: &mut UIStateGuard, id: usize| -> Result<()> {
            let uri = uris[id].clone();
            let context_id = match context_type {
                model::Type::Playlist => ContextId::Playlist(PlaylistId::from_uri(&uri)?),
                model::Type::Artist => ContextId::Artist(ArtistId::from_uri(&uri)?),
                model::Type::Album => ContextId::Album(AlbumId::from_uri(&uri)?),
                _ => {
                    return Ok(());
                }
            };

            send.send(ClientRequest::GetContext(context_id.clone()))?;

            ui.new_page(PageState::Browsing(context_id));

            Ok(())
        },
        |ui: &mut UIStateGuard| {
            ui.popup = None;
        },
    )
}

/// handles a key sequence for a generic list popup.
///
/// In addition the the application states and the key sequence,
/// the function requires to specify
/// - `n_items`: the number of items in the list
/// - `on_select_func`: the callback when selecting an item
/// - `on_choose_func`: the callback when choosing an item
/// - `on_close_func`: the callback when closing the popup
fn handle_key_sequence_for_list_popup(
    key_sequence: &KeySequence,
    state: &SharedState,
    ui: &mut UIStateGuard,
    n_items: usize,
    on_select_func: impl Fn(&mut UIStateGuard, usize),
    on_choose_func: impl Fn(&mut UIStateGuard, usize) -> Result<()>,
    on_close_func: impl Fn(&mut UIStateGuard),
) -> Result<bool> {
    let command = match state
        .keymap_config
        .find_command_from_key_sequence(key_sequence)
    {
        Some(command) => command,
        None => return Ok(false),
    };

    let popup = ui.popup.as_mut().unwrap();
    let current_id = popup.list_selected().unwrap();

    match command {
        Command::SelectPreviousOrScrollUp => {
            if current_id > 0 {
                popup.list_select(Some(current_id - 1));
                on_select_func(ui, current_id - 1);
            }
        }
        Command::SelectNextOrScrollDown => {
            if current_id + 1 < n_items {
                popup.list_select(Some(current_id + 1));
                on_select_func(ui, current_id + 1);
            }
        }
        Command::ChooseSelected => {
            on_choose_func(ui, current_id)?;
        }
        Command::ClosePopup => {
            on_close_func(ui);
        }
        _ => {
            return Ok(false);
        }
    };
    Ok(true)
}

/// handles a key sequence for a command shortcut help popup
fn handle_key_sequence_for_command_help_popup(
    key_sequence: &KeySequence,
    state: &SharedState,
    ui: &mut UIStateGuard,
) -> Result<bool> {
    let command = match state
        .keymap_config
        .find_command_from_key_sequence(key_sequence)
    {
        Some(command) => command,
        None => return Ok(false),
    };

    let offset = match ui.popup {
        Some(PopupState::CommandHelp(ref mut offset)) => offset,
        _ => unreachable!(),
    };
    match command {
        Command::ClosePopup => {
            ui.popup = None;
        }
        Command::SelectNextOrScrollDown => {
            *offset += 1;
        }
        Command::SelectPreviousOrScrollUp => {
            if *offset > 0 {
                *offset -= 1;
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// handles a key sequence for an action list popup
fn handle_key_sequence_for_action_list_popup(
    actions: Vec<Action>,
    key_sequence: &KeySequence,
    send: &mpsc::Sender<ClientRequest>,
    state: &SharedState,
    ui: &mut UIStateGuard,
) -> Result<bool> {
    handle_key_sequence_for_list_popup(
        key_sequence,
        state,
        ui,
        actions.len(),
        |_, _| {},
        |ui: &mut UIStateGuard, id: usize| -> Result<()> {
            let item = match ui.popup {
                Some(PopupState::ActionList(ref item, _)) => item,
                _ => unreachable!(),
            };

            match item {
                Item::Track(track) => match actions[id] {
                    Action::BrowseAlbum => {
                        if let Some(ref album) = track.album {
                            let uri = album.id.uri();
                            let context_id = ContextId::Album(AlbumId::from_uri(&uri)?);
                            send.send(ClientRequest::GetContext(context_id.clone()))?;
                            ui.new_page(PageState::Browsing(context_id));
                        }
                    }
                    Action::BrowseArtist => {
                        ui.popup = Some(PopupState::ArtistList(
                            track.artists.clone(),
                            new_list_state(),
                        ));
                    }
                    Action::AddTrackToPlaylist => {
                        let player = state.player.read().unwrap();
                        if let Some(ref user) = player.user {
                            let playlists = player
                                .user_playlists
                                .iter()
                                .filter(|p| p.owner.1 == user.id)
                                .cloned()
                                .collect();

                            ui.popup = Some(PopupState::UserPlaylistList(
                                PlaylistPopupAction::AddTrack(track.id.clone()),
                                playlists,
                                new_list_state(),
                            ));
                        }
                    }
                    Action::SaveToLibrary => {
                        send.send(ClientRequest::SaveToLibrary(item.clone()))?;
                        ui.popup = None;
                    }
                    Action::BrowseRecommendations => {
                        let seed = SeedItem::Track(track.clone());
                        send.send(ClientRequest::GetRecommendations(seed.clone()))?;
                        ui.new_page(PageState::Recommendations(seed, None));
                    }
                },
                Item::Album(album) => match actions[id] {
                    Action::BrowseArtist => {
                        ui.popup = Some(PopupState::ArtistList(
                            album.artists.clone(),
                            new_list_state(),
                        ));
                    }
                    Action::SaveToLibrary => {
                        send.send(ClientRequest::SaveToLibrary(item.clone()))?;
                        ui.popup = None;
                    }
                    _ => {}
                },
                Item::Artist(artist) => match actions[id] {
                    Action::SaveToLibrary => {
                        send.send(ClientRequest::SaveToLibrary(item.clone()))?;
                        ui.popup = None;
                    }
                    Action::BrowseRecommendations => {
                        let seed = SeedItem::Artist(artist.clone());
                        send.send(ClientRequest::GetRecommendations(seed.clone()))?;
                        ui.new_page(PageState::Recommendations(seed, None));
                    }
                    _ => {}
                },
                Item::Playlist(_) => {
                    if let Action::SaveToLibrary = actions[id] {
                        send.send(ClientRequest::SaveToLibrary(item.clone()))?;
                        ui.popup = None;
                    }
                }
            }
            Ok(())
        },
        |ui: &mut UIStateGuard| {
            ui.popup = None;
        },
    )
}