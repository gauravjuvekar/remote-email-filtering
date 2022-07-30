import base64


class XOauth2(object):
    def __init__(self, user, token):
        self.xoauth2_string = f'user={user}\x01auth=Bearer {token}\x01\x01'

    def authenticate_imap(self):
        return lambda _: self.xoauth2_string
