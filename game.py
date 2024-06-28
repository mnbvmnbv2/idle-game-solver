import numpy as np
import time


class Resource:
    def __init__(self, price, cost_mult, cost_increase_step, income) -> None:
        self.quantity = 0
        # price data
        self._price = price
        self._cost_mult = cost_mult
        self._cost_increase_step = cost_increase_step
        #
        self._one_income = income

    def buy_one(self):
        self.quantity += 1
        self._price = (self._price * self._cost_mult) + self._cost_increase_step

    def get_break_even(self):
        return self._price / self._one_income

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
        self.income_mult = 1
        self.resources = [
            Resource(price=4, cost_mult=1.07, cost_increase_step=0, income=3),
            Resource(price=200, cost_mult=1.15, cost_increase_step=0, income=200),
            Resource(price=6, cost_mult=1.25, cost_increase_step=0, income=5),
        ]
        self.step_ = 0

    @property
    def income(self):
        return sum(r.income for r in self.resources) * self.income_mult

    @property
    def owned(self):
        return [r.quantity for r in self.resources]

    @property
    def costs(self):
        return [r.price for r in self.resources]

    def get_values(self):
        # cps = self.INCOMES / self.costs
        break_even_times = [
            r.get_break_even() * self.income_mult for r in self.resources
        ]
        return break_even_times

    def buy(self, idx):
        if idx == -1:
            return False
        if self.money >= self.resources[idx].price:
            self.money -= self.resources[idx].price
            self.resources[idx].buy_one()
            return True
        return False

    def print_step(self, goal):
        print(
            f"Step: {self.step_:03}, "
            f"Money: {self.money:.6}, "
            f"Owned: {self.owned}, "
            f"Income: {self.income}, "
            f"Costs: {self.costs}, "
            f"Untill Goal: {self.time_untill(goal)}"
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

    def solve(self, goal, verbose=False):
        while self.money < goal:
            pretime = self.time_untill(goal)
            bought = self.buy(self.optimal_play(goal))
            while bought:
                bought = self.buy(self.optimal_play(goal))
            self.step(goal, verbose)
            posttime = self.time_untill(goal)
            assert posttime < pretime - 0.99
            # time.sleep(0.1)
        return self.step_

    def reset(self):
        for r in self.resources:
            r.quantity = 0
        self.money = self.start_money
        self.income_mult = 1

    def ascend(self):
        if self.money > 499:
            new_mult = self.money / 500
        else:
            new_mult = 1
        self.reset()
        self.income_mult = new_mult

    # def get_optimal_ascend(self):


def main():
    # game = Game()
    # game.solve(5000, verbose=True)
    for goal in [5000, 50000, 5e8]:
        game = Game()
        print(game.solve(goal))


if __name__ == "__main__":
    main()
