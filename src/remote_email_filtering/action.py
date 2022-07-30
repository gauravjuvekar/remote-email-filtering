import abc


class Action(abc.ABC):
    """A callable that does something with a Message"""

    def __init__(self):
        self.remote = None

    @abc.abstractmethod
    def __call__(self, message):
        pass
