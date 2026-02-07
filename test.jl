using Test

include("game2.jl")

@testset "Idle Solver Tests" begin

    r_cheap = Resource("Clicker", 0, q -> 10.0, q -> 1.0 * q)
    r_expensive = Resource("Factory", 0, q -> 100.0, q -> 20.0 * q)

    resources = [r_cheap, r_expensive]

    @testset "Basic Affordability" begin
        current_cash = 50.0
        current_yield = 1.0
        goal = 1000.0

        choice, time = find_fastest_path_to_goal(goal, current_cash, current_yield, resources, [0, 0])

        @test choice in [:buy, :wait]
    end

    @testset "Optimal Decision: No Brainer" begin
        r_best = Resource("Godly", 0, q -> 1.0, q -> 100.0 * q)
        r_worst = Resource("Trash", 0, q -> 100.0, q -> 1.0 * q)

        res_list = [r_best, r_worst]
        choice, time = find_fastest_path_to_goal(1000.0, 10.0, 1.0, res_list, [0, 0])
    end

    @testset "Budget Fitting" begin
        goal = 200.0
        cash = 10.0
        yield = 1.0

        choice, time = find_fastest_path_to_goal(goal, cash, yield, resources, [0, 0])

        @test time <= (goal - cash) / yield
    end
end