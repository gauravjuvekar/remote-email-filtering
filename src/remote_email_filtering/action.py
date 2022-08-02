"""
Basic building blocks for doing stuff with a
:class:`~remote_email_filtering.message.Message`
"""

import abc


class Action(abc.ABC):
    """
    A callable that does something with a :class:`~.message.Message`

    Instances will be called with :class:`~.message.Message` instances.
    The :attr:`remote` attribute will be set to a :class:`Remote` before
    calling.

    """

    def __init__(self):
        self.remote = None

    @abc.abstractmethod
    def __call__(self, msg) -> 'Iterable[Action]':
        """
        When called, an :class:`Action` should return an iterable of other
        :class:`Action` s that will be applied to the ``msg`` being processed.
        """
        pass


class Stop(Action):
    """
    Stop processing any futher :class:`Action` for the current
    :class:`~.message.Message`
    """

    def __call__(self, msg):
        raise StopIteration()


class Move(Action):
    def __init__(self, destination: tuple[str]):
        """
        Move the :class:`~.message.Message` to ``destination`` directory.

        :param tuple[str] destination: the destination directory on the server
        """
        super().__init__()
        self.destination = destination

    def __call__(self, msg):
        self.remote.move_message(msg, self.destination)
        return []
