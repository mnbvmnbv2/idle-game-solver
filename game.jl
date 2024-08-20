mutable struct Resource
    quantity::Int
    price::Float64
    initial_price::Float64
    cost_mult::Float64
    cost_increase_step::Float64
    one_income::Float64

    function Resource(price, cost_mult, cost_increase_step, income)
        new(0, price, cost_mult, cost_increase_step, income, income)
    end
end

mutable struct Game
    self.money = 150.0
    self.income_mult = 1.0
    self.resources = [
        Resource(4.0, 1.07, 0, 3),
        Resource(200.0, 1.15, 0, 200),
        Resource(6.0, 1.25, 0, 5),
    ]
    self.step = 0
    self.ascend_equilibrium = 2000.0

    function Game(money, income_mlt)
        new(0, money, income_mlt, [
            Resource(4.0, 1.07, 0, 3),
            Resource(200.0, 1.15, 0, 200),
            Resource(6.0, 1.25, 0, 5),
        ], 0, 2000.0)
    end
end

function income(self::Resource)
    return self.one_income * self.quantity
end

function income(self::Game)
    return sum(income(r) for r in self.resources) * self.income_mult
end

function buy_one(self::Resource)
    self.quantity += 1
    self.price = (self.price * self.cost_mult) + self.cost_increase_step
end

function get_break_even(self::Resource)
    return self.price / self.one_income
end

function reset(self::Resource)
    self.quantity = 0
    self.price = self.initial_price
end

function display_info(self::Resource)
    println("Resource(", self.quantity, ", ", self.price, ", ", self.one_income, ")")
end

function owned(self::Game)
    return [r.quantity for r in self.resources]
end

function costs(self::Game)
    return [r.price for r in self.resources]
end

function get_break_even_times(self::Game)
    return [
        get_break_even(r) / self.income_mult for r in self.resources
    ]
end

function buy(self::Game, idx::int)
    if idx == -1:
        return False
    if self.money >= self.resources[idx].price:
        self.money -= self.resources[idx].price
        self.resources[idx].buy_one()
        return True
    return False
end

function step(self::Game, goal::float = None, verbose::bool = False)
    self.money += self.income
    self.step_ += 1
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
end

function time_untill(self::Game, goal: float)
    if self.income == 0:
        return float("inf")
    adjusted_target = goal - self.money
    return adjusted_target / self.income
end

function reset(self::Game, income_mult: float = 1.0)
    for r in self.resources:
        r.reset()
    self.money = self.start_money
    self.income_mult = income_mult
end