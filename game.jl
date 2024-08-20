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
    money::Float64
    income_mult::Float64
    resources::Array{Resource}
    step::Int
    ascend_equilibrium::Float64

    function Game(money, income_mlt)
        new(money, income_mlt, [
            Resource(4.0, 1.07, 0, 3),
            Resource(200.0, 1.15, 0, 200),
            Resource(6.0, 1.25, 0, 5)
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

function buy(self::Game, idx::Int)
    if idx == -1
        return False
    end
    if self.money >= self.resources[idx].price
        self.money -= self.resources[idx].price
        self.resources[idx].buy_one()
        return True
    end
    return False
end

function step(self::Game)
    self.money += self.income
    self.step_ += 1
    println("Step: ",self.step_,"Money: ",self.money,"Income: ",self.income,"Owned: ",self.owned,"Costs: ",self.costs)
end

function time_untill(self::Game, goal::Float64)
    if self.income == 0
        return float("inf")
    end
    adjusted_target = goal - self.money
    return adjusted_target / self.income
end

function reset(self::Game, income_mult::Float64 = 1.0)
    for r in self.resources
        r.reset()
    end
    self.money = self.start_money
    self.income_mult = income_mult
end

function optimal_play(self::Game, goal::Float64)
    break_even_times = get_break_even_times(self)
    best_buys = argsort(break_even_times)
    worth_waiting_until_goal = break_even_times[best_buys[0]] > time_untill(self, goal)
    if worth_waiting_until_goal
        return -1
    end
    # iterate over the fastest break even buys
    num_candidates = len(best_buys)
    for i in num_candidates - 1
        curr_candidate = best_buys[i]
        next_candidate = best_buys[i + 1]
        no_cash = self.costs[curr_candidate] > self.money
        if no_cash
            enough_next_plus_this= self.costs[curr_candidate] + self.costs[next_candidate]
            next_worth = break_even_times[next_candidate] < time_untill(self, enough_next_plus_this)
            if not next_worth
                return -1
            end
            if next_worth
                continue
            end
        end
        if !no_cash
            break
        end
    end

    if curr_candidate == best_buys[-2] && next_worth
        curr_candidate = best_buys[-1]
    end
    if no_cash
        return -1
    end
    return curr_candidate
end

function non_ascend_solve(self::Game, goal::Float64)
    while self.money < goal
        while self.buy(self.optimal_play(goal))
            pass
        end
        self.step(goal, verbose)
    end
    return self.step_
end

function main()
    game = Game(150.0, 1.0)
    buy_one(game.resources[1])
    println("Done")
end

main()