import abc
import imaplib
import ssl

from . import auth


class Remote(abc.ABC):
    @abc.abstractmethod
    def list_dirs(self):
        pass


class Imap(Remote, auth.XOauth2):
    def __init__(self, host, **kwargs):
        super().__init__(**kwargs)
        self.connection = imaplib.IMAP4_SSL(
            host,
            ssl_context=ssl.create_default_context())

        self.connection.authenticate('XOAUTH2', self.authenticate_imap())

    def list_dirs(self):
        response = self.connection.list()
        assert response[0] == 'OK'
        return response[1]
