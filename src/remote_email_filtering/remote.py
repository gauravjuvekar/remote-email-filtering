# Copyright 2022, Gaurav Juvekar
# SPDX-License-Identifier: MIT
import abc
import itertools
import logging
import typing

import imapclient

from . import message, types

log = logging.getLogger(__name__)

imapclient.imaplib.Debug = 0


class Remote(abc.ABC):
    @abc.abstractmethod
    def list_dirs(self) -> typing.Iterable[types.Directory]:
        """
        List all ``Directory`` in the mailbox.
        """
        pass

    @abc.abstractmethod
    def list_messages(self, dir_: types.Directory) -> types.Uid:
        """
        List unique identifiers for all messages in ``dir_``. These identifiers
        must be unique for the entire mailbox.
        """
        pass

    @abc.abstractmethod
    def fetch_envelope(self, msg_id: types.Uid):
        """
        Fetch the envelope parsed from the email headers.
        """
        pass

    @abc.abstractmethod
    def fetch_multiple_envelopes(self, msg_ids: typing.Iterable[types.Uid]
            ) -> typing.Iterable[typing.Tuple[types.Uid, bytes]]:
        """
        Fetch multiple envelopes, attempting to batch them in least possible
        requests.
        """
        pass

    @abc.abstractmethod
    def fetch_body(self, msg_id: types.Uid):
        """
        Fetch the full email body.
        """
        pass

    def get_messages(self, dir_: types.Directory
                     ) -> typing.Iterable[message.Message]:
        """
        Get all messages in ``dir_``
        """
        list_msg = list(self.list_messages(dir_))
        for msg_id, envelope in zip(
            list_msg, self.fetch_multiple_envelopes(list_msg)
        ):
            yield message.Message(
                uid=msg_id, envelope=envelope, dir_=dir_, remote=self
            )

    @abc.abstractmethod
    def move_message_id(self, msg_id: types.Uid, target_dir: types.Directory
                        ) -> types.Uid:
        """
        Move ``msg_id`` to ``target_dir``.
        """
        pass

    def move_message(self, msg: message.Message, target_dir: types.Directory):
        """
        Move ``msg`` to ``taget_dir``.
        """
        new_uid = self.move_message_id(msg.uid, target_dir)
        msg.dir_ = target_dir
        msg.uid = new_uid


class Imap(Remote):
    def __init__(self, host, user, token, **kwargs):
        super().__init__(**kwargs)
        self.connection = imapclient.IMAPClient(host)
        self.connection.oauth2_login(user, access_token=token)

    def list_dirs(self):
        for flags, delim, name in self.connection.list_folders():
            name_components = tuple(name.split(delim.decode()))
            yield name_components

    def list_messages(self, dir_):
        self.connection.select_folder('/'.join(dir_))
        for uid in self.connection.search():
            # IMAP message uid are unique only within the directory. Create a
            # composite uid that contains the directory.
            yield (dir_, uid)

    def fetch_envelope(self, msg_id):
        dir_, uid = msg_id
        self.connection.select_folder('/'.join(dir_))
        ret = self.connection.fetch(uid, ['UID', 'ENVELOPE'])
        msg = ret[uid]
        return msg[b'ENVELOPE']

    def fetch_multiple_envelopes(self, msg_ids):
        for dir_, uids in itertools.groupby(msg_ids, key=lambda uid: uid[0]):
            self.connection.select_folder('/'.join(dir_))
            local_uids = [uid[1] for uid in uids]
            ret = self.connection.fetch(local_uids, ['UID', 'ENVELOPE'])
            yield from (ret[uid][b'ENVELOPE'] for uid in local_uids)

    def fetch_body(self, msg_id):
        dir_, uid = msg_id
        self.connection.select_folder('/'.join(dir_))
        ret = self.connection.fetch(uid, ['UID', 'RFC822'])
        msg = ret[uid]
        return msg[b'RFC822']

    def move_message_id(self, msg_id, target_dir):
        dir_, uid = msg_id
        self.connection.select_folder('/'.join(dir_))
        self.connection.move([uid], '/'.join(target_dir))
        return (target_dir, uid)
