import email
import email.header
import email.policy
import typing

from . import types

class Message(object):
    """
    An email message with convenient properties
    """
    def __init__(self, uid: types.Uid,
                 envelope, remote, dir_=None, rfc822_bytes=None):
        """
        :param uid: A unique identifier for a message within ``dir_``
        :param envelope: The envelope structure parsed from headers
        :param remote: A :class:`~.remote.Remote` used to lazy-load the body
        :param tuple[str] dir_: the mailbox directory that this email is in
        """
        self.uid = uid
        self.envelope = envelope._asdict()
        for field in ('cc', 'bcc', 'from_', 'reply_to', 'sender', 'to'):
            if not self.envelope[field]:
                self.envelope[field] = []
            self.envelope[field] = tuple((types.Address.from_imapclient(x)
                                          for x in self.envelope[field]))

        self.remote = remote
        self.dir_ = dir_
        self.raw = rfc822_bytes
        self._body = None
        if rfc822_bytes is not None:
            self._body = email.message_from_bytes(self.raw,
                                                  policy=email.policy.default)

    @property
    def body(self):
        if self._body is None:
            self.raw = self.remote.fetch_body(self.uid)
            self._body = email.message_from_bytes(self.raw,
                                                  policy=email.policy.default)
        return self._body

    @property
    def To(self):
        return self.envelope['to']

    @property
    def Cc(self):
        return self.envelope['cc']

    @property
    def From(self):
        return self.envelope['from_']

    @property
    def Recipients(self):
        return self.To + self.Cc

    @property
    def Subject(self):
        return self.envelope['subject']

    @property
    def _SaneSubject(self):
        ret = self.body['Subject']
        ret = email.header.decode_header(ret)[0]
        if ret[1] is not None:
            ret = ret[0].decode('ascii', errors='replace')
        else:
            ret = ret[0]
        ret = ret.replace('\n', '')
        return ret
