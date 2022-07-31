import email
import email.header


class Message(object):
    def __init__(self, rfc822_bytes):
        self.raw = rfc822_bytes
        self.mail = email.message_from_bytes(self.raw)

    @property
    def To(self):
        ret = self.mail['To']
        if ret is None:
            ret = ''
        return ret

    @property
    def Recipients(self):
        return ', '.join(_ for _ in (self.mail['To'], self.mail['CC'])
                         if _ is not None)

    @property
    def Subject(self):
        return self.mail['Subject']

    @property
    def SaneSubject(self):
        ret = self.mail['Subject']
        ret = email.header.decode_header(ret)[0]
        if ret[1] is not None:
            ret = ret[0].decode('ascii', errors='replace')
        else:
            ret = ret[0]
        ret = ret.replace('\n', '')
        return ret

    @property
    def From(self):
        return self.mail['From']
