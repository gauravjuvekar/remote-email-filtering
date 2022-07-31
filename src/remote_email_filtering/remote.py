import abc
import imapclient


class Remote(abc.ABC):
    @abc.abstractmethod
    def list_dirs(self):
        pass


class Imap(Remote):
    def __init__(self, host, user, token, **kwargs):
        super().__init__(**kwargs)
        self.connection = imapclient.IMAPClient(host)
        self.connection.oauth2_login(user, access_token=token)

    def list_dirs(self):
        for flags, delim, name in self.connection.list_folders():
            name_components = tuple(name.split(delim.decode()))
            yield name_components
