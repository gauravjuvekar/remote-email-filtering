#!/usr/bin/env python
import google.auth.transport.requests
import google_auth_oauthlib.flow
import json
import requests


def initial_authorize(secrets):
    flow = google_auth_oauthlib.flow.InstalledAppFlow.from_client_config(
        secrets,
        scopes=['https://mail.google.com/'])

    credentials = flow.run_local_server(host='localhost', port=8900)
    return credentials.to_json()


def main():
    import argparse

    parser = argparse.ArgumentParser(
        "Convert an initial client_secrets.json to an authorzied secrets file")
    parser.add_argument(
        "CLIENT_SECRETS",
        type=argparse.FileType(mode='r'),
        help="client_secrets.json file")
    parser.add_argument(
        "OUTPUT_JSON",
        type=str,
        help="client_secrets.json file")
    args = parser.parse_args()

    secrets = json.loads(args.CLIENT_SECRETS.read())
    output = initial_authorize(secrets)

    with open(args.OUTPUT_JSON, 'w') as f:
        f.write(json.dumps(output))


if __name__ == '__main__':
    main()
