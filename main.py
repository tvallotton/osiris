from dataclasses import dataclass, field
import random


@dataclass
class Passenger: 
    destination : int
    start       : int = field(default_factory=lambda: MINUTE)


    def time_elapsed(self): 
        return MINUTE - self.start
    

@dataclass
class Station: 
    passengers : list[Passenger]
    position   : int
    
   
    def receive_passengers(self, subway):
        destinations = set(subway.stations) - set([self.position])
        n_passengers = random.randint(0, 10)
        
        for _ in range(n_passengers):
            destination = random.sample(list(destinations), 1)[0]
            passenger   = Passenger(destination)
            self.passengers.append(passenger)
        
        print(f"time: {MINUTE:^ 5} station: {self.position:^ 5} passengers: {len(self.passengers)}")

   
@dataclass
class Train: 
    position      : int
    direction     : int
    passengers    : list[Passenger] = field(default_factory=lambda: [])
    should_depart : bool = True


    def take_passengers(self, station: Station):
        stay_in_station = []
        
        for passenger in station.passengers: 
            passenger_dir = self.position < passenger.destination
            train_dir     = self.direction == 1
            

            if train_dir == passenger_dir:
                DEPARTURE_TIME.append(passenger.time_elapsed()) 
                self.passengers.append(passenger)
            else: 
                stay_in_station.append(passenger)
                
        station.passengers = stay_in_station
        
        

    def leave_passengers(self): 
        passengers = []
        for passenger in self.passengers:
            if passenger.destination == self.position: 
                ELAPSED_TIMES.append(passenger.time_elapsed())
            else: 
                passengers.append(passenger)

        self.passengers = passengers


    def advance(self, subway): 

        is_station = subway.stations.get(self.position)

        if is_station and not self.should_depart: 
            self.leave_passengers()
            self.take_passengers(subway.stations[self.position])
            self.should_depart = True
            return

        self.should_depart = False
        self.position += self.direction
        switch_direction = self.position % (len(subway.positions) - 1) == 0 

        if switch_direction: 
            self.direction = -self.direction

        assert self.position < len(subway.positions), self.position
        assert 0 <= self.position, self.position
        

class Subway: 
    def __init__(self, E: list[int], trains: list[Train]):
        self.positions = E
        self.stations  = {}
        self.trains    = trains

        for position, is_station in enumerate(E): 
            if is_station: 
                station = Station([], position)
                self.stations[position] = station


    def left_length(self, departure, destination): 
        return (len(self.positions) + destination - departure) % len(self.positions)
    

    def next(self): 
        for station in self.stations.values(): 
            station.receive_passengers(self)

        for train in self.trains: 
            train.advance(self)
            

MINUTE = 0
ELAPSED_TIMES = []
DEPARTURE_TIME = []
TRAINS = [
    Train(0, 1), 
    Train(16, 1),
    Train(32, 1),
    Train(16, -1), 
    Train(32, -1), 
    Train(47, -1), 
]
E = [
    1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0,
    0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1]
subway = Subway(E, TRAINS)

for i in range(60*10):
    subway.next()
    MINUTE += 1

print("mean time:", sum(ELAPSED_TIMES) / len(ELAPSED_TIMES))