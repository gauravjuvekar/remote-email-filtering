import datetime
import time
from pprint import pp


def start_filtering(remote,
                    filter_map=dict(),
                    interval=datetime.timedelta(seconds=5)):
    while True:
        pp(list(remote.list_dirs()))
        time.sleep(interval.seconds)
