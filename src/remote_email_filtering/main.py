# Copyright 2022, Gaurav Juvekar
# SPDX-License-Identifier: MIT
import datetime
import itertools
import logging
import threading
import time
import typing

from . import types

log = logging.getLogger(__name__)


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
          count=float('inf'),
          stop_event=threading.Event()):
    """
    Start applying :class:`~.action.Action` s to all messages in specified
    directories.

    :param Dict[Directory, List[Action]] dir_actions:
       :class:`~.action.Action` to apply to all messages in the directory
    :param interval: duration to wait after each pass
    :param count: number of times to loop through all directories
    :param stop_event: Event object that safely exits from a loop before
    `count` expires
    """
    validity = dict((k, False) for k in dir_actions.keys())

    while count > 0 and not stop_event.is_set():
        for dir_ in remote.list_dirs():
            if stop_event.is_set():
                break

            if dir_ not in validity:
                continue

            new_validity = remote.dir_validity(dir_)
            if validity[dir_] == new_validity:
                log.debug(f"No new messages in {dir_}")
                continue
            else:
                log.debug(f"{dir_} has new messages")
                validity[dir_] = new_validity

            for message in remote.get_messages(dir_):
                if stop_event.is_set():
                    break
                pipeline(message, dir_actions[dir_])

        count -= 1
        stop_event.wait(interval.seconds)
