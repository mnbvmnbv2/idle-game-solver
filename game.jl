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

function income(self::Resource)
    return self.one_income * self.quantity
end

function display_info(self::Resource)
    println("Resource(", self.quantity, ", ", self.price, ", ", self.one_income, ")")
end
