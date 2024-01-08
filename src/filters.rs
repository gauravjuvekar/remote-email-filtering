use maybe_owned::MaybeOwned;
use rand;

use crate::actions;
use crate::types;

pub struct Print {
    pub message: String,
    pub some_state: u32,
}

impl actions::LogicAction for Print {
    fn process_msg(
        &self,
        msg: &types::Message,
        folder: &types::Folder,
    ) -> Vec<actions::Action> {
        println!("Acting on {} in {}", msg.id, folder.path[0]);

        let mut ret: Vec<actions::Action> = vec![];
        if rand::random() {
            ret.push(actions::Action::Logic(Box::new(Print {
                message: "dynamically created".to_string(),
                some_state: rand::random(),
            })));
        }
        ret
    }
}

impl Drop for Print {
    fn drop(&mut self) {
        println!("Print {} was dropped", self.message);
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

    let mut flags;
    let mut dest_dir;
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
                out_actions = function.process_msg(&message, folder);
            }
            actions::Action::Flags(change) => {
                flags = change;
                break;
            }
            actions::Action::Move(dest) => {
                dest_dir = dest;
                break;
            }
            actions::Action::Stop => break,
        }

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
            println!("Processing {}", folder.path[0]);
            process_folder(&folder, actions);
            println!("done");
        }
    }
}
