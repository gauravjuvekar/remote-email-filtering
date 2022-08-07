import collections
import re
import typing


class Address(collections.namedtuple('Address', 'name mailbox host',
                                     defaults=(None, None, None))):
    @classmethod
    def from_imapclient(cls, addr):
        return cls(name=addr.name, mailbox=addr.mailbox, host=addr.host)

    def re_match(self, addr):
        sname, smbox, shost = self
        if sname is None:
            sname = b''

        if any(x is None for x in (addr.name, addr.mailbox, addr.host)):
            return False
        return all((re.fullmatch(addr.name, sname),
                    re.fullmatch(addr.mailbox, smbox),
                    re.fullmatch(addr.host, shost)))


"""
A directory on the mail server made up of the path components of the directory
"""
Directory = typing.Tuple[str]

Uid = typing.Hashable
