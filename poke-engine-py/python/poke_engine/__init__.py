from dataclasses import dataclass

# noinspection PyUnresolvedReferences
from ._poke_engine import (
    State,
    Side,
    SideSlot,
    SideConditions,
    VolatileStatusDurations,
    Pokemon,
    Move,
    generate_instructions,
    calculate_damage,
    mcts,
)


@dataclass
class MctsSideResult:
    """
    Result of a Monte Carlo Tree Search for a single side

    :param move_choice: The move that was chosen
    :type move_choice: str
    :param total_score: The total score of the chosen move
    :type total_score: float
    :param visits: The number of times the move was chosen
    :type visits: int
    """

    move_choice: str
    total_score: float
    visits: int


@dataclass
class MctsResult:
    """
    Result of a Monte Carlo Tree Search

    :param side_one: Result for side one
    :type side_one: list[MctsSideResult]
    :param side_two: Result for side two
    :type side_two: list[MctsSideResult]
    :param total_visits: Total number of monte carlo iterations
    :type total_visits: int
    """

    side_one: list[MctsSideResult]
    side_two: list[MctsSideResult]
    total_visits: int

    @classmethod
    def _from_rust(cls, rust_result):
        return cls(
            side_one=[
                MctsSideResult(
                    move_choice=i.move_choice,
                    total_score=i.total_score,
                    visits=i.visits,
                )
                for i in rust_result.side_one
            ],
            side_two=[
                MctsSideResult(
                    move_choice=i.move_choice,
                    total_score=i.total_score,
                    visits=i.visits,
                )
                for i in rust_result.side_two
            ],
            total_visits=rust_result.iteration_count,
        )


def monte_carlo_tree_search(state: State, duration_ms: int = 1000) -> MctsResult:
    """
    Perform monte-carlo-tree-search on the given state and for the given duration

    :param state: the state to search through
    :type state: State
    :param duration_ms: time in milliseconds to run the search
    :type duration_ms: int
    :return: the result of the search
    :rtype: MctsResult
    """
    return MctsResult._from_rust(mcts(state, duration_ms))
