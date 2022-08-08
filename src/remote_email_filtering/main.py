# Copyright 2022, Gaurav Juvekar
# SPDX-License-Identifier: MIT
import datetime
import itertools
import time
import typing

from . import types


def pipeline(message, actions):
    actions = iter(actions)
    while True:
        try:
            action = next(actions)
        except StopIteration:
            break

        action.remote = message.remote
        try:
            further = action(message)
            if further is None:
                raise Exception(msg=f'Action: {action} returned None')
        except StopIteration:
            return
        actions = itertools.chain(further, actions)


def start(remote,
          dir_actions: typing.Dict[types.Directory, typing.List['Action']] = dict(),
          interval=datetime.timedelta(seconds=5),
          count=float('inf')):
    """
    Start applying :class:`~.action.Action` s to all messages in specified
    directories.

    :param Dict[Directory, List[Action]] dir_actions:
       :class:`~.action.Action` to apply to all messages in the directory
    :param interval: duration to wait after each pass
    :param count: number of times to loop through all directories
    """
    while count > 0:
        for dir_ in remote.list_dirs():
            if dir_ in dir_actions:
                for message in remote.get_messages(dir_):
                    pipeline(message, dir_actions[dir_])
        count -= 1

        time.sleep(interval.seconds)
