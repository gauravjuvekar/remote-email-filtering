import abc
import imapclient

from . import message


class Remote(abc.ABC):
    @abc.abstractmethod
    def list_dirs(self):
        pass

    @abc.abstractmethod
    def list_message_ids(self, dir):
        pass

    @abc.abstractmethod
    def fetch_message(self, message_id):
        pass

    def get_messages(self, dir):
        for msg_id in self.list_message_ids(dir):
            message = self.fetch_message(msg_id)
            yield message


class Imap(Remote):
    def __init__(self, host, user, token, **kwargs):
        super().__init__(**kwargs)
        self.connection = imapclient.IMAPClient(host)
        self.connection.oauth2_login(user, access_token=token)

    def list_dirs(self):
        for flags, delim, name in self.connection.list_folders():
            name_components = tuple(name.split(delim.decode()))
            yield name_components

    def list_message_ids(self, dir):
        self.connection.select_folder('/'.join(dir))
        return self.connection.search()

    def fetch_message(self, msg_id):
        msg = self.connection.fetch(msg_id, ['FLAGS', 'INTERNALDATE',
                                             'ENVELOPE', 'RFC822'])
        msg = msg[msg_id]
        msg = message.Message(msg[b'RFC822'])
        return msg
