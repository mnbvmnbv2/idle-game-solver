import sys
import math
import numpy as np
import matplotlib.pyplot as plt
import time
from dataclasses import dataclass
from enum import Enum, auto


sys.setrecursionlimit(100000)

num_calls = 0
all_calls = []

start_time = time.perf_counter()


@dataclass
class AscendCombo:
    step: float
    mult: float


@dataclass
class Returner:
    time: float
    ascends: list[float]
    ascend_combo: list[AscendCombo]


class Strategy(Enum):
    RETIRE = auto()  # untill goal
    SAVING = auto()  # untill we can buy more
    BUYING = auto()


class SavingStrategy(Enum):
    UNTIL_GOAL = -1


class BestState:
    def __init__(
        self, time: float = float("inf"), ascends: list[AscendCombo] | None = None
    ):
        self.time = time
        self.ascends = ascends if ascends is not None else [AscendCombo(0.0, 1.0)]
        self.best_mults: dict[float, float] = {0.0: 1.0}

    def update(self, time: float, ascends: list[AscendCombo]) -> None:
        if time < self.time:
            self.time = time
            self.ascends = ascends

    def update_mult(self, step: float, mult: float) -> None:
        if step not in self.best_mults:
            self.best_mults[step] = mult
        else:
            self.best_mults[step] = max(self.best_mults[step], mult)


class Resource:
    def __init__(self, price, cost_mult, cost_increase_step, income) -> None:
        self.quantity = 0
        # price data
        self._price = price
        self._initial_price = price
        self._cost_mult = cost_mult
        self._cost_increase_step = cost_increase_step
        #
        self._one_income = income

    def buy_one(self):
        self.quantity += 1
        self._price = (self._price * self._cost_mult) + self._cost_increase_step

    def get_break_even(self):
        return self._price / self._one_income

    def reset(self):
        self.quantity = 0
        self._price = self._initial_price

    @property
    def income(self):
        return self._one_income * self.quantity

    @property
    def price(self):
        return self._price

    def __repr__(self) -> str:
        return f"Resource({self.quantity}, {self.price:.2f}, {self._one_income})"


class Game:
    def __init__(self) -> None:
        self.start_money = 150.0
        self.money = self.start_money
        self.income_mult = 1.0
        self.resources = [
            Resource(price=4.0, cost_mult=1.07, cost_increase_step=0, income=3),
            Resource(price=200.0, cost_mult=1.15, cost_increase_step=0, income=200),
            Resource(price=6.0, cost_mult=1.25, cost_increase_step=0, income=5),
        ]
        self.step_ = 0
        self.ascend_equilibrium = 2000.0

    @property
    def income(self) -> float:
        return sum(r.income for r in self.resources) * self.income_mult

    @property
    def owned(self) -> list[int]:
        return [r.quantity for r in self.resources]

    @property
    def costs(self) -> list[float]:
        return [r.price for r in self.resources]

    def get_break_even_times(self) -> list[float]:
        break_even_times = [
            r.get_break_even() / self.income_mult for r in self.resources
        ]
        return break_even_times

    def buy(self, idx: int) -> bool:
        if self.money >= self.resources[idx].price:
            self.money -= self.resources[idx].price
            self.resources[idx].buy_one()
            return True
        return False

    def step(
        self,
        num_steps: int = 1,
        goal: float | None = None,
        verbose: bool = False,
    ) -> None:
        self.money += self.income * num_steps
        self.step_ += num_steps
        if self.step_ % 100000 == 0:
            if verbose:
                untill_goal = f"Untill Goal: {self.time_untill(goal)}" if goal else ""
                print(
                    f"Step: {self.step_:03}, "
                    f"Money: {self.money:.6}, "
                    f"Owned: {self.owned}, "
                    f"Income: {self.income}, "
                    f"Costs: {self.costs}, ",
                    untill_goal,
                )

    def print_important_metrics(self) -> None:
        print(f"Step: {self.step_:03}")
        print(f"Money: {self.money:.6}")
        print(f"Owned: {self.owned}")
        print(f"Income: {self.income}")
        print(f"Costs: {self.costs}\n")

    def time_untill(self, goal: float) -> float:
        if self.income == 0:
            return float("inf")
        adjusted_target = goal - self.money
        return adjusted_target / self.income

    def reset(self, income_mult: float = 1.0) -> None:
        for r in self.resources:
            r.reset()
        self.money = self.start_money
        self.income_mult = income_mult

    def ascend(self) -> None:
        new_mult = self.get_ascend_value(self.money)
        if new_mult > self.income_mult:
            self.reset(new_mult)
        else:
            self.reset(self.income_mult)

    def get_ascend_value(self, money: float) -> float:
        if money > self.ascend_equilibrium:
            return money / self.ascend_equilibrium
        return 1.0

    def optimal_play(self, goal: float) -> int | SavingStrategy:
        """Returns the index of the optimal play - best roi or, -1 if it is worth to wait instead.

        NOTE: It is actually not the optimal play, as in some situations it could be better to buy multiple
        of a worse roi resource if it "fits" into the current budget.
        """
        break_even_times = self.get_break_even_times()
        best_buys = np.argsort(break_even_times)
        worth_waiting_until_goal = break_even_times[best_buys[0]] > self.time_untill(
            goal
        )
        if worth_waiting_until_goal:
            return SavingStrategy.UNTIL_GOAL

        # iterate over the fastest break even buys
        for curr_candidate, next_candidate in zip(
            best_buys, np.concatenate((best_buys[1:], best_buys[-1:]))
        ):
            can_buy = self.costs[curr_candidate] < self.money
            if can_buy:
                return curr_candidate
            # cannot buy
            next_worth = break_even_times[next_candidate] < self.time_untill(
                self.costs[curr_candidate] + self.costs[next_candidate]
            )
            if not next_worth:
                return curr_candidate
        if not can_buy:
            return curr_candidate
        return curr_candidate

    def non_ascend_solve(self, goal: float, verbose: bool = False) -> int:
        while self.money < goal:
            # pretime = self.time_untill(goal)
            buy_strategy = self.optimal_play(goal)
            match buy_strategy:
                case SavingStrategy.UNTIL_GOAL:
                    self.step(math.ceil(self.time_untill(goal)))
                    break
                case _:
                    while True:
                        bought = self.buy(buy_strategy)
                        if bought:
                            break
                        until_enough = math.ceil(
                            self.time_untill(self.costs[buy_strategy])
                        )
                        self.step(until_enough)
                    continue
            # self.step(1, goal, verbose)
            # posttime = self.time_untill(goal)
            # assert posttime < pretime - 0.99
        return self.step_

    def solve(
        self,
        goal: float,
        start_time: float,
        best_state: BestState | None = None,
    ) -> Returner:
        """Returns the time it takes to reach the goal"""
        if best_state is None:
            best_state = BestState()
        global num_calls
        global all_calls
        time_untill_goal, money = ghost_solve(goal, self.income_mult)
        assert money >= goal
        all_calls.append(
            (goal, start_time, best_state.time, time.perf_counter(), time_untill_goal)
        )
        num_calls += 1
        # check if faster with ascend some intervals along the way
        if goal > self.ascend_equilibrium:
            viable_ascends = []
            # find if any point before goal is reached with current multiplier is worth ascending
            # we don't bother checking if it is possible to reach with a limit of 1 step
            end = math.floor(time_untill_goal) - 2
            # check in sort of binary order, cutting parts that arae not worth ascending
            to_check = list(range(2, end))
            while to_check:
                # pick 1/2 point
                idx = len(to_check) // 2
                i = to_check.pop(idx)
                # skip if not worth ascending as the the is too late
                # Note this is inside loop because it can be updated inside the loop
                if best_state.time - start_time - 1 < i:
                    # remove all to_check that are larger than i
                    to_check = [x for x in to_check if x < i]  # to_check[:idx]
                    continue
                money = max_reachable_in(i, self.income_mult)
                mult_at_i = self.get_ascend_value(money)
                # if we don't gain substancially from ascending
                if mult_at_i < self.income_mult * 1.1:
                    # remove all to_check that are smaller than i
                    to_check = [x for x in to_check if x > i]
                    continue
                # check if we have a lower multiplier and less time left than any of the steps in what we
                # know to be the best

                # update mult
                current_total_step = start_time + i
                best_state.update_mult(current_total_step, mult_at_i)

                breakout = False
                for ascend_combo in best_state.ascends:
                    time_left = best_state.time - start_time - i
                    ascend_time_left = best_state.time - ascend_combo.step
                    less_time = time_left <= ascend_time_left
                    less_mult = mult_at_i <= ascend_combo.mult
                    if less_mult and less_time:
                        breakout = True
                if breakout:
                    continue

                # if we are lacking on mult we skip
                if (
                    current_total_step in best_state.best_mults
                    and mult_at_i < best_state.best_mults[current_total_step]
                ):
                    continue
                # recursively check if ascending is worth it
                ghost_game = self.__class__()
                ghost_game.income_mult = mult_at_i
                returner = ghost_game.solve(goal, start_time + i, best_state)
                # if ascending is worth it we add to candidate list
                total_time = returner.time + i + start_time
                if total_time < time_untill_goal:
                    # print(total_time)
                    viable_ascends.append((returner.time + i, i, mult_at_i, returner))
                # update best time
                if total_time < best_state.time:
                    best_state.update(total_time, returner.ascend_combo)
            # sort by time used
            if viable_ascends:
                viable_ascends.sort(key=lambda x: x[0])
                return Returner(
                    viable_ascends[0][0],
                    [start_time] + viable_ascends[0][3].ascends,
                    [
                        AscendCombo(
                            start_time + viable_ascends[0][1], viable_ascends[0][2]
                        )
                    ]
                    + returner.ascend_combo,
                )

        # if not worth ascending we return the time it took to reach the goal
        # print(time_untill_goal + start_time)
        return Returner(
            time_untill_goal,  # remaining time untill goal
            [start_time],
            [AscendCombo(start_time, self.income_mult)],
        )


def max_reachable_in(steps: int, mult: float = 1.0) -> float:
    """Using similar logic to optimal play"""
    ghost_game = Game()
    ghost_game.income_mult = mult

    for steps_left in range(steps, 0, -1):
        strategy = Strategy.BUYING
        while strategy == Strategy.BUYING:
            break_even_times = ghost_game.get_break_even_times()
            best_buys = np.argsort(break_even_times)

            worth_waiting_until_goal = break_even_times[best_buys[0]] > steps_left
            if worth_waiting_until_goal:
                strategy = Strategy.RETIRE
                break

            # iterate over the fastest break even buys
            for curr_candidate, next_candidate in zip(
                best_buys, np.concatenate((best_buys[1:], best_buys[-1:]))
            ):
                can_buy = ghost_game.costs[curr_candidate] <= ghost_game.money
                if can_buy:
                    break
                # roi within this candidate
                next_worth = break_even_times[next_candidate] < ghost_game.time_untill(
                    ghost_game.costs[curr_candidate] + ghost_game.costs[next_candidate]
                )
                if not next_worth:
                    break
            # we could not buy the current candidate, but the current one is worth it
            # or we can buy the current candidate
            # try to buy either way
            bought = ghost_game.buy(curr_candidate)
            if not bought:
                strategy = Strategy.SAVING

        match strategy:
            case Strategy.RETIRE:
                ghost_game.step(steps_left)
                break
            case Strategy.SAVING:
                time_until_enough = math.ceil(
                    ghost_game.time_untill(ghost_game.costs[curr_candidate])
                )
                ghost_game.step(time_until_enough)
    return ghost_game.money


def ghost_solve(goal: float, mult: float = 1.0) -> tuple[int, float]:
    ghost_game = Game()
    ghost_game.income_mult = mult
    return ghost_game.non_ascend_solve(goal), ghost_game.money


def optimal_based_on_ascends(ascends: list[int], final_step: int) -> float:
    game = Game()
    mult = 1.0
    step = 0
    for i, ascend in enumerate(ascends):
        money = max_reachable_in(ascend - step, mult)
        mult = game.get_ascend_value(money)
        step = ascend
        print(i, ascend, money, mult)
    money = max_reachable_in(final_step - step, mult)
    return money


def main():
    # Normal
    solutions = []
    pre = time.monotonic()
    for i in range(1, 100):
        game = Game()
        ret = game.solve(1657 * i, 0)
        solutions.append(ret.time)
    post = time.monotonic()
    print(post - pre)

    # plot
    plt.plot(solutions)
    plt.show()


def speed():
    pre = time.monotonic()
    game = Game()
    # game.non_ascend_solve(2000012784500, verbose=True)
    # max_reachable_in(2000000)
    returner = game.solve(100_000_000, 0)
    num_steps = returner.time
    ascends = returner.ascends
    post = time.monotonic()
    print(post - pre)
    print(f"{num_steps=}")
    print(f"{num_calls=}")

    print(ascends)
    # visualize all_calls
    all_calls.sort(key=lambda x: x[0])
    plt.plot([x[1] for x in all_calls], label="start_time")
    plt.plot([x[2] for x in all_calls], label="best_time")
    plt.plot([x[3] - start_time for x in all_calls], label="time")
    plt.plot([x[4] for x in all_calls], label="time_untill_goal")
    plt.legend()
    plt.show()


def speed2():
    pre = time.monotonic()
    game = Game()
    sol = game.non_ascend_solve(200_000_000_000_000_000, verbose=True)
    print(sol)
    post = time.monotonic()
    print(post - pre)


def speed3():
    pre = time.monotonic()
    sol = max_reachable_in(200_000_000_000_000)
    print(sol)
    post = time.monotonic()
    print(post - pre)


def test():
    game = Game()
    game.optimal_play(200)


if __name__ == "__main__":
    speed()

    # res = optimal_based_on_ascends([0, 12, 19], 25)
    # print(res)

    # time_untill_goal, _ = ghost_solve(2_000_000, 1.0)
    # print(time_untill_goal)

    # print(max_reachable_in(10, 3.17))
