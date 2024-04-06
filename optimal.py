import numpy as np
import time


class Game:
    def __init__(self) -> None:
        self.income_mult = 1
        self.COST_INCREASE = np.array([1.07, 1.15, 1.25])  # Multiplier
        self.INCOMES = np.array([3, 300, 5]) * self.income_mult
        self.COST_INCREASE_STEP = np.array([1.1, 1.2, 1.3])
        self.GOAL = 1000

        self.costs = np.array([4, 300, 6], dtype=float)
        self.owned = np.array([0, 0, 0])
        self.money = 200.0
        self.step_ = 0

    @property
    def income(self):
        return np.sum(self.owned * self.INCOMES)

    def get_values(self):
        # cps = self.INCOMES / self.costs
        break_even_times = self.costs / self.INCOMES
        return break_even_times

    def buy(self, idx):
        if idx == -1:
            return False
        if self.money >= self.costs[idx]:
            self.owned[idx] += 1
            self.money -= self.costs[idx]
            self.costs[idx] = self.costs[idx] * self.COST_INCREASE[idx]
            return True
        return False

    def print_step(self, goal):
        print(
            f"Step: {self.step_:03}, "
            f"Money: {self.money:.6}, "
            f"Owned: {self.owned}, "
            f"Income: {self.income}, "
            f"Costs: {self.costs}, "
            f"CPS: {self.time_untill(goal)}"
            # f"values: {self.get_values()}"
        )

    def get_da_cash(self):
        self.money += self.income

    def step(self, goal, verbose=False):
        self.get_da_cash()
        self.step_ += 1
        if verbose:
            self.print_step(goal)

    def time_untill(self, goal):
        if self.income == 0:
            return float("inf")
        adjusted_target = goal - self.money
        return adjusted_target / self.income

    def optimal_play(self, goal):
        break_even_times = self.get_values()
        best_buys = np.argsort(break_even_times)
        worth_waiting_until_goal = break_even_times[best_buys[0]] > self.time_untill(
            goal
        )
        if worth_waiting_until_goal:
            return -1
        for curr_candidate, next_candidate in zip(best_buys, best_buys[1:]):
            no_cash = self.costs[curr_candidate] > self.money
            # print(no_cash)
            if no_cash:
                next_worth = break_even_times[next_candidate] < self.time_untill(
                    self.costs[next_candidate] + self.costs[next_candidate]
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


def main(goal):
    game = Game()
    while game.money < goal:
        pretime = game.time_untill(goal)
        bought = game.buy(game.optimal_play(goal))
        while bought:
            bought = game.buy(game.optimal_play(goal))
        game.step(goal)
        posttime = game.time_untill(goal)
        assert posttime < pretime - 0.99
        # time.sleep(0.1)
    return game.step_


if __name__ == "__main__":
    for goal in [5000, 50000, 5e8]:
        print(main(goal))
