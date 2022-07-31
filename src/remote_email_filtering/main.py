import datetime
import time
from pprint import pp


def filter_message(message, filters):
    pass


def start_filtering(remote,
                    filter_map=dict(),
                    interval=datetime.timedelta(seconds=5)):
    while True:
        for dir in remote.list_dirs():
            if dir in filter_map:
                for message in remote.get_messages(dir):
                    filter_message(message, filter_map[dir])

        time.sleep(interval.seconds)
