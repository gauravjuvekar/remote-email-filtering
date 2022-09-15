# Copyright 2022, Gaurav Juvekar
# SPDX-License-Identifier: MIT
import abc
import itertools
import logging
import typing

import exchangelib
import imapclient
import imapclient.response_types
import oauthlib
import oauthlib.oauth2

from . import message, types

log = logging.getLogger(__name__)

imapclient.imaplib.Debug = 0


class Remote(abc.ABC):
    @abc.abstractmethod
    def is_dir_updated(self, dir_: types.Directory, watermark):
        """
        Returns (True/False, new watermark) if there are changes in dir_
        compared to the watermark.
        """
        pass

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

    @abc.abstractmethod
    def fetch_flags(self, msg_id: types.Uid) -> typing.Set[typing.ByteString]:
        """
        Get flags associated with a ``msg_id``
        """
        pass

    @abc.abstractmethod
    def add_flags(self, msg_id: types.Uid,
                  flags: typing.Set[typing.ByteString]
                  ) -> typing.Set[typing.ByteString]:
        """
        Add and associated flags with a ``msg_id``
        """
        pass

    @abc.abstractmethod
    def remove_flags(self, msg_id: types.Uid,
                     flags: typing.Set[typing.ByteString]
                     ) -> typing.Set[typing.ByteString]:
        """
        Remove flags associated with a ``msg_id``
        """
        pass


class Imap(Remote):
    def __init__(self, host, user, token, **kwargs):
        super().__init__(**kwargs)
        self.connection = imapclient.IMAPClient(host)
        self.connection.oauth2_login(user, access_token=token)

    def is_dir_updated(self, dir_, watermark=None):
        ret = self.connection.select_folder('/'.join(dir_))
        new_watermark = (ret[b'UIDVALIDITY'], ret[b'UIDNEXT'])
        return watermark != new_watermark, new_watermark

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

    def fetch_flags(self, msg_id):
        dir_, uid = msg_id
        self.connection.select_folder('/'.join(dir_))
        flags = self.connection.get_flags([uid])
        return set(flags[uid])

    def add_flags(self, msg_id, flags):
        dir_, uid = msg_id
        self.connection.select_folder('/'.join(dir_))
        return set(self.connection.add_flags([uid], flags)[uid])

    def remove_flags(self, msg_id, flags):
        dir_, uid = msg_id
        self.connection.select_folder('/'.join(dir_))
        return set(self.connection.remove_flags([uid], flags)[uid])


class Ews(Remote):
    def __init__(self, host, user, token, **kwargs):
        super().__init__(**kwargs)

        self.connection = exchangelib.Account(
            primary_smtp_address=user,
            config=exchangelib.Configuration(
                server=host,
                credentials=exchangelib.OAuth2AuthorizationCodeCredentials(access_token=token)),
            autodiscover=False,
            access_type=exchangelib.DELEGATE)

        self.toplevel = self.connection.msg_folder_root

    def _resolve_dir(self, parts):
        start = self.connection.msg_folder_root
        for part in parts:
            start = start / part
        return start

    def _unresolve_dir(self, dir_obj):
        toplevel_strip = len(self.toplevel.parts)
        return tuple((x.name for x in dir_obj.parts[toplevel_strip:]))

    def _resolve_msg_obj(self, msg_id):
        dir_, msg_id = msg_id
        dir_obj = self._resolve_dir(dir_)
        msg = dir_obj.get(**msg_id)
        return msg

    def is_dir_updated(self, dir_, watermark=None):
        dir_ = self._resolve_dir(dir_)
        sync = list(dir_.sync_items(only_fields=['id']))
        return bool(sync), None

    def list_dirs(self):
        toplevel_strip = len(self.toplevel.parts)
        for dir_ in self.toplevel.walk():
            yield self._unresolve_dir(dir_)

    def list_messages(self, dir_):
        dir_obj = self._resolve_dir(dir_)
        for msgid in dir_obj.all().values('id', 'changekey'):
            yield (dir_, msgid)

    def fetch_envelope(self, msg_id):
        msg = self._resolve_msg_obj(msg_id)
        envelope = imapclient.response_types.Envelope(
            date=msg.datetime_received,
            subject=msg.subject.encode('utf-8'),
            from_=tuple([types.Address.from_exchangelib(msg.author)]),
            sender=(tuple([types.Address.from_exchangelib(msg.sender)])
                    if msg.sender else None),
            reply_to=tuple([types.Address.from_exchangelib(x) for x in
                            (msg.reply_to if msg.reply_to else [])]),
            to=tuple([types.Address.from_exchangelib(x) for x in
                      (msg.to_recipients if msg.to_recipients else [])]),
            cc=tuple([types.Address.from_exchangelib(x) for x in
                      (msg.cc_recipients if msg.cc_recipients else [])]),
            bcc=tuple([types.Address.from_exchangelib(x) for x in
                       (msg.bcc_recipients if msg.bcc_recipients else [])]),
            in_reply_to=msg.in_reply_to,
            message_id=msg.message_id)
        return envelope

    def fetch_multiple_envelopes(self, msg_ids):
        for msg_id in msg_ids:
            yield self.fetch_envelope(msg_id)

    def fetch_body(self, msg_id):
        msg = self._resolve_msg_obj(msg_id)
        return msg.mime_content

    def move_message_id(self, msg_id, target_dir):
        msg = self._resolve_msg_obj(msg_id)
        target = self._resolve_dir(target_dir)
        msg.move(target)
        return (self._unresolve_dir(msg.folder),
                {'id': msg.id, 'changekey': msg.changekey})

    FAKE_CATEGORIES = set([
        r'\Seen',
    ])

    def fetch_flags(self, msg_id):
        msg = self._resolve_msg_obj(msg_id)
        flags = msg.categories
        if flags is None:
            flags = set()
        else:
            flags = set(flags)
        if msg.is_read:
            flags |= set([r'\Seen'])
        return flags

    def change_flags(self, msg_id, flags, op):
        msg = self._resolve_msg_obj(msg_id)
        existing = self.fetch_flags(msg_id)
        new = op(existing, set(flags))
        if new == existing:
            return new

        msg.is_read = r'\Seen' in new
        msg.categories = list(new - self.FAKE_CATEGORIES)
        msg.save()
        return new

    def add_flags(self, msg_id, flags):
        return self.change_flags(msg_id, flags, op=lambda x, y: x | y)

    def remove_flags(self, msg_id, flags):
        return self.change_flags(msg_id, flags, op=lambda x, y: x - y)
