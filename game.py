import sys
import math
import numpy as np
import matplotlib.pyplot as plt


sys.setrecursionlimit(100000)


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
        self.ascend_equilibrium = 500.0

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
        if idx == -1:
            return False
        if self.money >= self.resources[idx].price:
            self.money -= self.resources[idx].price
            self.resources[idx].buy_one()
            return True
        return False

    def get_da_cash(self) -> None:
        self.money += self.income

    def step(self, goal: float | None = None, verbose: bool = False) -> None:
        self.get_da_cash()
        self.step_ += 1
        if verbose:
            self.print_step(goal)

    def print_step(self, goal: float | None):
        untill_goal = f"Untill Goal: {self.time_untill(goal)}" if goal else ""
        print(
            f"Step: {self.step_:03}, "
            f"Money: {self.money:.6}, "
            f"Owned: {self.owned}, "
            f"Income: {self.income}, "
            f"Costs: {self.costs}, ",
            untill_goal,
        )

    def time_untill(self, goal: float) -> float:
        if self.income == 0:
            return float("inf")
        adjusted_target = goal - self.money
        return adjusted_target / self.income

    def reset(self) -> None:
        for r in self.resources:
            r.reset()
        self.money = self.start_money
        self.income_mult = 1.0

    def ascend(self) -> None:
        new_mult = self.get_ascend_value(self.money)
        self.reset()
        self.income_mult = new_mult

    def get_ascend_value(self, money: float) -> float:
        if money > self.ascend_equilibrium:
            return money / self.ascend_equilibrium
        return 1.0

    def optimal_play(self, goal: float) -> int:
        """Returns the index of the optimal play - best roi or, -1 if it is worth to wait instead.

        NOTE: It is actually not the optimal play, as in some situations it could be better to buy multiple
        of a worse roi resource if it "fits" into the current budget.

        NOTE 2: Could return -2 if it is worth to wait and then skip calling this function in the main loop.

        NOTE 3: Need to check if worth waiting untill goal is also not optimal.
        """
        break_even_times = self.get_break_even_times()
        best_buys = np.argsort(break_even_times)
        worth_waiting_until_goal = break_even_times[best_buys[0]] > self.time_untill(
            goal
        )
        if worth_waiting_until_goal:
            return -1
        # iterate over the fastest break even buys
        for curr_candidate, next_candidate in zip(best_buys, best_buys[1:]):
            no_cash = self.costs[curr_candidate] > self.money
            if no_cash:
                next_worth = break_even_times[next_candidate] < self.time_untill(
                    self.costs[curr_candidate] + self.costs[next_candidate]
                )
                if not next_worth:
                    return -1
                if next_worth:
                    continue
            if not no_cash:
                break

        if no_cash:
            return -1
        return curr_candidate

    def max_reachable_in(self, steps: int, mult: float = 1.0) -> float:
        """Using similar logic to optimal play"""
        ghost_game = self.__class__()
        ghost_game.reset()
        ghost_game.income_mult = mult

        steps_left = steps
        while steps_left > 0:
            buying = True
            while buying:
                break_even_times = ghost_game.get_break_even_times()
                best_buys = np.argsort(break_even_times)

                # iterate over the fastest break even buys
                for curr_candidate, next_candidate in zip(best_buys, best_buys[1:]):
                    if break_even_times[curr_candidate] > steps_left:
                        buying = False
                        break
                    no_cash = ghost_game.costs[curr_candidate] > ghost_game.money
                    if no_cash:
                        should_wait = break_even_times[next_candidate] > steps_left
                        roi_within_this_candidate = break_even_times[
                            next_candidate
                        ] < ghost_game.time_untill(
                            ghost_game.costs[curr_candidate]
                            + ghost_game.costs[next_candidate]
                        )
                        next_worth = not should_wait and roi_within_this_candidate
                        if not next_worth:
                            buying = False
                        if next_worth:
                            continue
                    if not no_cash:
                        break
                if no_cash:
                    buying = False
                if buying:
                    buying = ghost_game.buy(curr_candidate)
            ghost_game.step()
            steps_left -= 1
        return ghost_game.money

    def non_ascend_solve(self, goal: float, verbose: bool = False) -> int:
        while self.money < goal:
            pretime = self.time_untill(goal)
            while self.buy(self.optimal_play(goal)):
                pass
            self.step(goal, verbose)
            posttime = self.time_untill(goal)
            assert posttime < pretime - 0.99
        return self.step_

    def ghost_solve(self, goal: float, mult: float = 1.0) -> tuple[int, float]:
        ghost_game = self.__class__()
        ghost_game.reset()
        ghost_game.income_mult = mult
        return ghost_game.non_ascend_solve(goal), ghost_game.money

    def solve_old(self, goal, verbose=False):
        time_untill_goal, _ = self.ghost_solve(goal, self.income_mult)
        # check if faster with ascend some intervals along the way
        if goal > self.ascend_equilibrium:
            interval_points = [
                goal * 0.03,
                goal * 0.06,
                goal * 0.1,
            ]
            ascend_times = []
            for interval_point in interval_points:
                to_ascend, ghost_money = self.ghost_solve(
                    interval_point, self.income_mult
                )
                from_ascend, _ = self.ghost_solve(
                    goal, self.get_ascend_value(ghost_money)
                )
                ascend_times.append(to_ascend + from_ascend)
            if min(ascend_times) < time_untill_goal:
                self.solve_old(interval_points[np.argmin(ascend_times)])
                self.ascend()

        return self.non_ascend_solve(goal, verbose)

    def solve(self, goal: float, limit: float | None = None) -> float:
        """Returns the time it takes to reach the goal"""
        time_untill_goal = self.non_ascend_solve(goal)
        # print(f"{time_untill_goal=}, {goal=}")
        if time_untill_goal <= 1:
            return time_untill_goal
        if limit and time_untill_goal > limit:
            return time_untill_goal
        # check if faster with ascend some intervals along the way
        if goal > self.ascend_equilibrium:
            viable_ascends = []
            # find if any point before goal is reached with current multiplier is worth ascending
            for i in range(1, math.floor(time_untill_goal)):
                money = self.max_reachable_in(i, self.income_mult)
                mult_at_i = self.get_ascend_value(money)
                if mult_at_i < 1.1:
                    continue
                ghost_game = self.__class__()
                ghost_game.reset()
                ghost_game.income_mult = mult_at_i
                steps_used_on_rest = ghost_game.solve(goal, time_untill_goal - i)
                # if ascending is worth it
                if steps_used_on_rest + i < time_untill_goal:
                    viable_ascends.append((steps_used_on_rest + i, i, money))
            # sort by time used
            viable_ascends.sort(key=lambda x: x[0])
            if viable_ascends:
                return viable_ascends[0][0]

        # if not worth ascending
        return time_untill_goal


def main():
    # Normal
    # game = Game()
    # game.solve(365719, verbose=True)

    times = []
    goals = range(500, int(5e5), 50003)
    for goal in goals:
        print(goal)
        game = Game()
        solved_time = game.solve(goal)
        times.append(solved_time)
    plt.plot(list(goals), times, label="Time")
    plt.show()


if __name__ == "__main__":
    main()
