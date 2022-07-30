#!/usr/bin/env python
import dateparser
import datetime
import google.auth.transport.requests
import google.oauth2.credentials
import json
import requests


def refresh(secrets, not_after=datetime.datetime.now(datetime.timezone.utc)):
    expiry = dateparser.parse(secrets['expiry'])
    if expiry > not_after:
        return

    credentials = google.oauth2.credentials.Credentials.from_authorized_user_info(secrets)
    credentials.refresh(google.auth.transport.requests.Request())
    return json.loads(credentials.to_json())


def main():
    import argparse

    parser = argparse.ArgumentParser(
        "Refresh an OAuth2 access token in-place")
    parser.add_argument(
        "AUTHORIZED_SECRETS",
        type=argparse.FileType(mode='r'),
        help="authorized_secret.json file")
    parser.add_argument(
        "--output",
        type=str,
        help="write refreshed access token to a different file")

    def duration_type(str):
        ret = dateparser.parse(str).astimezone()
        if ret is None:
            raise argparse.ArgumentTypeError(
                "{str} could not be interpreted as a duration")
        return ret

    parser.add_argument(
        "--validity",
        type=duration_type,
        default="30min",
        help="refresh only if existing access token expires before this duration")
    args = parser.parse_args()

    if args.output is None:
        args.output = args.AUTHORIZED_SECRETS.name

    secrets = json.loads(args.AUTHORIZED_SECRETS.read())
    args.AUTHORIZED_SECRETS.close()

    output = refresh(secrets, not_after=args.validity)

    if output is not None:
        with open(args.output, 'w') as f:
            f.write(json.dumps(output))


if __name__ == '__main__':
    main()
