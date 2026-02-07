struct Resource{F<:Function,G<:Function}
    name::String
    cost_fn::F   # Function: f(x) -> cost of the next unit
    yield_fn::G  # Function: g(x) -> total production per second
end

using Parameters
@kwdef struct GameState
    time::Float64 = 0.0
    money::Float64 = 0.0
    income::Float64 = 0.0
    inventory::Vector{Int} # How many of each resource we have
end

# r1 = Resource(
#     "Clicker",
#     q -> 10 * 1.1^q,
#     q -> 2 * q
# )
# r2 = Resource(
#     "Factory",
#     q -> 100 * 1.2^q,
#     q -> q >= 5 ? (10 * q * 3) : (10 * q)
# )

# resources = [r1, r2]
# budget = 1000.0
