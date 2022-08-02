#!/usr/bin/env python3
import json
import os
from pprint import pp
import sys

sys.path.insert(0, os.path.abspath('../src'))

import remote_email_filtering
import remote_email_filtering.remote
import remote_email_filtering.main
import remote_email_filtering.action
from remote_email_filtering.action import Action, Stop


def print_envelope(msg):
    pp(msg.envelope)
    return []


def main():
    import argparse

    parser = argparse.ArgumentParser(
        description="List top level directories over IMAP")

    parser.add_argument("ACCESS_TOKEN", type=argparse.FileType('r'),
                        help="json file with oauth2 access token")
    parser.add_argument("USER", type=str)
    parser.add_argument("HOST", type=str)

    args = parser.parse_args()
    creds = json.loads(args.ACCESS_TOKEN.read())

    remote = remote_email_filtering.remote.Imap(host=args.HOST,
                                                user=args.USER,
                                                token=creds['token'])
    filters = {
        ('INBOX',): [print_envelope, Stop()],
    }

    remote_email_filtering.main.start(remote, filters, count=1)


if __name__ == '__main__':
    main()
