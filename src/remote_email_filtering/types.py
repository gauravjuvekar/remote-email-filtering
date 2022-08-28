# Copyright 2022, Gaurav Juvekar
# SPDX-License-Identifier: MIT
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

        oname, omailbox, ohost = tuple(
            x if x is not None else rb'.*'
            for x in (addr.name, addr.mailbox, addr.host))

        return all((re.fullmatch(oname, sname),
                    re.fullmatch(omailbox, smbox),
                    re.fullmatch(ohost, shost)))


"""
A directory on the mail server made up of the path components of the directory
"""
Directory = typing.Tuple[str]

Uid = typing.Hashable
