use maybe_owned::MaybeOwned;
use std::collections::HashSet;
use tracing::{info, info_span};

use crate::actions;
use crate::types;

pub struct DebugPrint;

impl actions::LogicAction for DebugPrint {
    fn process_msg(
        &self,
        msg: &types::Message,
        folder: &types::Folder,
    ) -> Vec<actions::Action> {
        info!("Acting on {} in {}", msg.id, folder.path[0]);
        std::thread::sleep(std::time::Duration::from_secs(1));

        vec![]
    }
}

fn process_message(
    message: types::Message,
    folder: &types::Folder,
    actions: &Vec<actions::Action>,
) -> () {
    let borrowed_actions: Box<
        dyn Iterator<Item = MaybeOwned<actions::Action>>,
    > = {
        let ret = actions
            .iter()
            .map(|e| MaybeOwned::<actions::Action>::Borrowed(e))
            .into_iter();
        Box::new(ret)
    };

    struct Accumulate {
        delta_flags: Option<actions::ChangeFlags>,
        dest_dir: Option<types::Folder>,
        cache: bool,
        cache_string: Option<String>,
        invalidate_list: HashSet<String>,
    }

    let mut accumulated = Accumulate {
        delta_flags: None,
        dest_dir: None,
        cache: false,
        cache_string: None,
        invalidate_list: HashSet::new(),
    };

    let mut it = borrowed_actions.into_iter();
    loop {
        let contain_owned_action: actions::Action;
        let next_action: &actions::Action = match it.next() {
            Some(MaybeOwned::Owned(a)) => {
                contain_owned_action = a;
                &contain_owned_action
            }
            Some(MaybeOwned::Borrowed(a)) => a,
            None => break,
        };

        let out_actions;
        match next_action {
            actions::Action::Logic(function) => {
                out_actions = function.process_msg(&message, folder)
            }
            actions::Action::Flags(change) => {
                accumulated.delta_flags = Some(change.clone());
                break;
            }
            actions::Action::Move(dest) => {
                accumulated.dest_dir = Some(dest.clone());
                break;
            }
            actions::Action::Cache(key) => {
                accumulated.cache = true;
                accumulated.cache_string = key.clone();
                break;
            }
            actions::Action::InvalidateCache(key) => {
                accumulated.invalidate_list.insert(key.clone());
                break;
            }
            actions::Action::Stop => break,
        };

        it = Box::new(
            out_actions
                .into_iter()
                .map(|e| MaybeOwned::<actions::Action>::Owned(e))
                .chain(it),
        );
    }
    // TODO process accumulated
}

fn process_folder(
    folder: &types::Folder,
    actions: &Vec<actions::Action>,
) -> () {
    for message in folder.list_messages() {
        process_message(message, folder, actions);
    }
}

pub fn mainloop(filter_spec: &types::FilterSpec) {
    loop {
        for (folder, actions) in filter_spec {
            info_span!("process_folder", folder = folder.path[0]).in_scope(
                || {
                    process_folder(&folder, actions);
                },
            );
        }
    }
}
