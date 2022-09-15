#!/usr/bin/env python
import msal


def main(params, cache_path, username):
    cache = msal.SerializableTokenCache()
    try:
        with open(cache_path, "r") as f:
            cache.deserialize(f.read())
    except:
        pass

    app = msal.PublicClientApplication(client_id=params['client_id'],
                                       authority=params['authority'],
                                       token_cache=cache)
    accounts = app.get_accounts(username)
    if not accounts:
        device_flow = app.initiate_device_flow(scopes=params['scope'])
        print(device_flow['message'])
        result = app.acquire_token_by_device_flow(device_flow)

    account = app.get_accounts(username)[0]
    result = app.acquire_token_silent(
        scopes=params['scope'],
        account=account,
        force_refresh=True)

    if cache.has_state_changed:
        with open(cache_path, "w") as f:
            f.write(cache.serialize())
    return result


if __name__ == '__main__':
    import argparse
    import json
    parser = argparse.ArgumentParser(
        description="Authorize and save refresh token in cache")
    parser.add_argument("PARAMS_JSON", type=str,
                        help="Non-secret params for the application")
    parser.add_argument("SECRET_JSON", type=str,
                        help="Output file to store tokens into")
    parser.add_argument("USERNAME", type=str)

    args = parser.parse_args()
    with open(args.PARAMS_JSON, 'r') as f:
        params = json.load(f)

    result = main(params, args.SECRET_JSON, args.USERNAME)
    if result is None or 'access_token' not in result:
        raise Exception("Failed to refresh")
