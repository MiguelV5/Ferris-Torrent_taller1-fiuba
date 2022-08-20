var timesInJson = [];
var connectionsInJson = [];
var completedInJson = [];

var arrayTimes = [];
var arrayConnections = [];
var arrayCompleted = [];

var howFarInTime = 1;
var leapsInTime = "Hours"

var lineChart;

function isInTime(time,betweenTime) {
  let firstTime = new Date(betweenTime.getTime());
  let lastTime = new Date(betweenTime.getTime());
  if (leapsInTime == "Hours"){
    lastTime.setHours(betweenTime.getHours() + 1);
  } else {
    lastTime.setMinutes(betweenTime.getMinutes() + 1);
  }

  if (time >= firstTime && time < lastTime) {
      return true
  } else {
      return false
  }
}

function searchConnections() {
  arrayTimes = [];
  arrayConnections = [];
  arrayCompleted = [];

  let cantConnectedNow = 0;
  let cantCompletedNow = 0;
  let pos = 0;
  let now = new Date();
  let startingTime = new Date();
  startingTime.setHours(now.getHours() - howFarInTime);

  while (new Date(timesInJson[pos]) < startingTime){
    pos += 1;
    if (pos < timesInJson.length) {
      cantConnectedNow = connectionsInJson[pos];
      cantCompletedNow = completedInJson[pos];
    }
  }
  
  while (startingTime < now) {
    let keepSearching = pos < timesInJson.length;
    while (keepSearching) {
      let timeInJson = timesInJson[pos];
      if (isInTime(new Date(timeInJson), startingTime)){
        cantConnectedNow = connectionsInJson[pos];
        cantCompletedNow = completedInJson[pos];
        pos = pos + 1;
        keepSearching = pos < timesInJson.length;
      } else {
        keepSearching = false;
      }
    }

    arrayTimes.push(new Date(startingTime.getTime()));
    arrayConnections.push(cantConnectedNow);
    arrayCompleted.push(cantCompletedNow);

    if (leapsInTime == "Hours"){
      startingTime.setHours(startingTime.getHours() + 1);
    } else {
      startingTime.setMinutes(startingTime.getMinutes() + 1);
    }
  }
}

fetch('database.json')
  .then(response => response.json())
  .then(result => {
    timesInJson = result.times;
    connectionsInJson = result.connections;
    completedInJson = result.completed;

    searchConnections();

    lineChart = new Chart("lineGraph", {
      type: "line",
      data: {
        labels: arrayTimes,
        datasets: [{
          label: 'Conexiones activas',
          lineTension: 0,
          borderColor: 'rgba(0,0,255,1)',
          backgroundColor: 'rgba(0,0,255,0.3)',
          fill: true,
          data: arrayConnections
        },
        {
          label: 'Conexiones completas',
          lineTension: 0,
          borderColor: 'rgba(255,0,0,1)',
          backgroundColor: 'rgba(255,0,0,0.3)',
          fill: true,
          data: arrayCompleted
        }
      ]
      },
      options: {
        responsive: true,
        legend: {display: true},
        scales: {
          xAxes: [{
            type: 'time',
            distribution: 'linear',
            time: {
              displayFormats: {
                'hour': 'ddd HH:mm a',
                'day': 'MMM DD',
                'week': 'MMM DD',
                'month': 'DD MMM YYYY',
                'quarter': 'MMM YYYY',
                'year': 'MMM YYYY',
             }
            }
          }],
          yAxes: [{
            ticks: {
              beginAtZero: true
            }
        }]
        }
      }
    });
});

function alertLeapsTime() {
  let select = document.getElementById('leapsTime');
  let text = select.options[select.selectedIndex].text;
  if (text == 'In hours'){
    leapsInTime = "Hours";
  } else {
    leapsInTime = "Minutes";
  }
  searchConnections();
  lineChart.data.labels = arrayTimes
  lineChart.data.datasets[0].data = arrayConnections
  lineChart.data.datasets[1].data = arrayCompleted
  lineChart.update();
}
function alertLongTime() {
  let select = document.getElementById('howLongTime');
  let text = select.options[select.selectedIndex].text;
  if (text == 'Last hour') {
    howFarInTime = 1;
  } else if (text == 'Last five hours') {
    howFarInTime = 5;
  } else if (text == 'Last day') {
    howFarInTime = 24;
  } else {
    howFarInTime = 72;
  }
  searchConnections();
  lineChart.data.labels = arrayTimes
  lineChart.data.datasets[0].data = arrayConnections
  lineChart.data.datasets[1].data = arrayCompleted
  lineChart.update();
}
