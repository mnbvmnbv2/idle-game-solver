mutable struct Resource
    quantity::Int
    price::Float64
    initial_price::Float64
    cost_mult::Float64
    cost_increase_step::Float64
    one_income::Float64

    function Resource(price, cost_mult, cost_increase_step, income)
        new(0, price, price, cost_mult, cost_increase_step, income)
    end
end

mutable struct Game
    money::Float64
    income_mult::Float64
    resources::Array{Resource}
    step::Int
    ascend_equilibrium::Float64

    function Game()
        new(150, 1.0, [
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
        return false
    end
    if self.money >= self.resources[idx].price
        self.money -= self.resources[idx].price
        buy_one(self.resources[idx])
        return true
    end
    return false
end

function step(self::Game)
    self.money += income(self)
    self.step += 1
    if self.step % 100000 == 0
        println("Step: ",self.step,", Money: ",self.money,", Income: ",income(self),", Owned: ",owned(self),", Costs: ",costs(self))
    end
end

function time_untill(self::Game, goal::Float64)
    if income(self) == 0
        return float(typemax(Float64))
    end
    adjusted_target = goal - self.money
    return adjusted_target / income(self)
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
    best_buys = sortperm(break_even_times)
    worth_waiting_until_goal = break_even_times[best_buys[1]] > time_untill(self, goal)
    if worth_waiting_until_goal
        return -1
    end
    # create variables prior to loop
    next_worth = false
    no_cash = false
    # iterate over the fastest break even buys
    num_candidates = size(best_buys)[1]
    for i = 1:num_candidates - 1
        curr_candidate = best_buys[i]
        next_candidate = best_buys[i + 1]
        no_cash = self.resources[curr_candidate].price > self.money
        if no_cash
            enough_next_plus_this= self.resources[curr_candidate].price + self.resources[next_candidate].price
            next_worth = break_even_times[next_candidate] < time_untill(self, enough_next_plus_this)
            if !next_worth
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

    if curr_candidate == best_buys[end-1] && next_worth
        curr_candidate = best_buys[end]
    end
    if no_cash
        return -1
    end
    return curr_candidate
end

function non_ascend_solve(self::Game, goal::Float64)
    while self.money < goal
        buying = true
        while buying
            buying = buy(self, optimal_play(self, goal))
        end
        step(self)
    end
    return self.step
end

function ghost_solve(goal::Float64, mult::Float64 = 1.0)
    ghost_game = Game()
    reset(ghost_game, mult)
    return ghost_game.non_ascend_solve(goal), ghost_game.money
end

function max_reachable_in(steps::Int, mult::Float64 = 1.0)
    ghost_game = Game()
    ghost_game.income_mult = mult

    for steps_left = steps:-1:1
        buying = true
        while buying
            break_even_times = get_break_even_times(ghost_game)
            best_buys = sortperm(break_even_times)

            next_worth = false
            can_buy = false
            # iterate over the fastest break even buys
            num_candidates = size(best_buys)[1]
            for i = 1:num_candidates - 1
                curr_candidate = best_buys[i]
                next_candidate = best_buys[i + 1]
                if break_even_times[curr_candidate] > steps_left
                    buying = false
                    break
                end
                can_buy = costs(ghost_game)[curr_candidate] <= ghost_game.money
                if can_buy
                    break
                end
                if !can_buy
                    # roi within this candidate
                    next_worth = break_even_times[next_candidate] < time_untill(ghost_game, costs(ghost_game)[curr_candidate] + costs(ghost_game)[next_candidate])
                    if !next_worth
                        buying = false
                    end
                    if next_worth
                        continue
                    end
                end
            end
            # check if loop finished and next_worth (we should buy last candidate)
            # this is due to zip loop ending before we can check the last candidate
            if curr_candidate == best_buys[end-1] && next_worth
                curr_candidate = best_buys[end]
            end
            if !can_buy
                buying = false
            end
            if buying
                buying = buy(ghost_game, curr_candidate)
            end
        end
        step(ghost_game)
    end
    return ghost_game.money
end

# function solve(
#     self::Game, goal::Float64, start_time::Float64, best_time::Float64 = float(typemax(Float64))
# )
#     """Returns the time it takes to reach the goal"""
#     time_untill_goal, _ = ghost_solve(goal, self.income_mult)
#     # check if faster with ascend some intervals along the way
#     if goal > self.ascend_equilibrium:
#         viable_ascends = []
#         # find if any point before goal is reached with current multiplier is worth ascending
#         # we don't bother checking if it is possible to reach with a limit of 1 step
#         end = math.floor(time_untill_goal) - 2
#         # check in reverse order so we can break once we reach non viable ascends
#         for i in range(end, 1, -1):
#             # skip if not worth ascending as the the is too late
#             # added this in loop for ease of understanding rather than checking before loop
#             if best_time - start_time - 1 < i:
#                 continue
#             money = max_reachable_in(i, self.income_mult)
#             mult_at_i = self.get_ascend_value(money)
#             # if we don't gain substancially from ascending
#             if mult_at_i < self.income_mult * 1.1:
#                 break
#             # recursively check if ascending is worth it
#             ghost_game = self.__class__()
#             ghost_game.income_mult = mult_at_i
#             steps_used_on_rest = ghost_game.solve(goal, start_time + i, best_time)
#             # if ascending is worth it we add to candidate list
#             if steps_used_on_rest + i < time_untill_goal:
#                 viable_ascends.append((steps_used_on_rest + i, i, money))
#             # update best time
#             if (new_time := start_time + i + steps_used_on_rest) < best_time:
#                 best_time = new_time
#         # sort by time used
#         if viable_ascends:
#             viable_ascends.sort(key=lambda x: x[0])
#             # viable_ascend_list.append(viable_ascends)
#             return viable_ascends[0][0]

#     # if not worth ascending we return the time it took to reach the goal
#     return time_untill_goal
# end


function main()
    # game = Game()
    # done_step = non_ascend_solve(game, 2000012784500.0)
    # println("Done ", done_step)
    money = max_reachable_in(2000000)
    println("Money: ", money)
end

elapsed_time = @elapsed begin
    main()
end

println("Elapsed time: ", elapsed_time)