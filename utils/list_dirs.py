#!/usr/bin/env python3
import json
import os
import sys

sys.path.insert(0, os.path.abspath('../src'))


def main():
    import argparse
    import remote_email_filtering
    import remote_email_filtering.remote
    import remote_email_filtering.main

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
        ('INBOX',): [lambda x: x, ],
    }

    remote_email_filtering.main.start_filtering(remote, filters)


if __name__ == '__main__':
    main()
