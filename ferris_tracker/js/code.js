const zero = 0;
const one = 1;
const two = 2;
const five = 5;
const aDay = 24;
const threeDays = 72;

var cantTorrentsInJson = zero;
var timesInJson = [];
var connectionsInJson = [];
var completedInJson = [];

var arrayTimes = [];
var arrayConnections = [];
var arrayCompleted = [];
var arrayTorrents = [];

var howFarInTime = one;
var leapsInTime = "Hours"

var lineChart;

function isInTime(time,betweenTime) {
  let firstTime = new Date(betweenTime.getTime());
  let lastTime = new Date(betweenTime.getTime());
  if (leapsInTime == "Hours"){
    lastTime.setHours(betweenTime.getHours() + one);
  } else {
    lastTime.setMinutes(betweenTime.getMinutes() + one);
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
  arrayTorrents = [];

  let cantConnectedNow = zero;
  let cantCompletedNow = zero;
  let pos = zero;
  let now = new Date();
  let startingTime = new Date();
  startingTime.setHours(now.getHours() - howFarInTime);

  while (new Date(timesInJson[pos]) < startingTime){
    pos += one;
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
        pos = pos + one;
        keepSearching = pos < timesInJson.length;
      } else {
        keepSearching = false;
      }
    }

    arrayTimes.push(new Date(startingTime.getTime()));
    arrayTorrents.push(cantTorrentsInJson);
    arrayConnections.push(cantConnectedNow);
    arrayCompleted.push(cantCompletedNow);

    if (leapsInTime == "Hours"){
      startingTime.setHours(startingTime.getHours() + one);
    } else {
      startingTime.setMinutes(startingTime.getMinutes() + one);
    }
  }
}

fetch('database.json')
  .then(response => response.json())
  .then(result => {
    cantTorrentsInJson = result.torrents;
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
          lineTension: zero,
          borderColor: 'rgba(0,0,255,1)',
          backgroundColor: 'rgba(0,0,255,0.3)',
          fill: true,
          data: arrayConnections
        },
        {
          label: 'Conexiones completas',
          lineTension: zero,
          borderColor: 'rgba(255,0,0,1)',
          backgroundColor: 'rgba(255,0,0,0.3)',
          fill: true,
          data: arrayCompleted
        },
        {
          label: 'Cantidad Torrents',
          lineTension: zero,
          borderColor: 'rgba(0,255,0,1)',
          backgroundColor: 'rgba(0,255,0,0.3)',
          fill: true,
          data: arrayTorrents
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
  lineChart.data.labels = arrayTimes;
  lineChart.data.datasets[zero].data = arrayConnections;
  lineChart.data.datasets[one].data = arrayCompleted;
  lineChart.data.datasets[two].data = arrayTorrents;
  lineChart.update();
}
function alertLongTime() {
  let select = document.getElementById('howLongTime');
  let text = select.options[select.selectedIndex].text;
  if (text == 'Last hour') {
    howFarInTime = one;
  } else if (text == 'Last five hours') {
    howFarInTime = five;
  } else if (text == 'Last day') {
    howFarInTime = aDay;
  } else {
    howFarInTime = threeDays;
  }
  searchConnections();
  lineChart.data.labels = arrayTimes;
  lineChart.data.datasets[zero].data = arrayConnections;
  lineChart.data.datasets[one].data = arrayCompleted;
  lineChart.data.datasets[two].data = arrayTorrents;
  lineChart.update();
}
