pub struct BoxAndWhiskerPlot {
    pub parent_id: String,
    pub width: f64,
    pub data: Vec<Vec<f64>>,
    pub series_labels: Vec<String>,
    pub x_axis_label: String,
    pub draw_gridlines: bool,
    pub draw_legend: bool,
    pub draw_grey_background: bool,
    pub bar_width: f64,
    pub bar_gap: f64,
    pub title: String,
    pub show_title: bool,
}

impl BoxAndWhiskerPlot {
    pub fn get_svg(&self) -> String {
        let data = format!("{:?}", self.data);
        let series_labels2 = format!("{:?}", self.series_labels);
        let mut s = String::new();
        s.push_str(&format!(
            "
    <script>
      var plot = {{
        data: {},
        seriesLabels: {},
        xAxisLabel: \"{}\",
        width: {},
        drawGridlines: {},
        drawLegend: {},
        drawGreyBackground: {},
        parentId: \"{}\",
        barWidth: {},
        barGap: {},
        title: \"{}\",
        showTitle: {}
      }};",
            data,
            series_labels2,
            self.x_axis_label,
            self.width,
            self.draw_gridlines,
            self.draw_legend,
            self.draw_grey_background,
            self.parent_id,
            self.bar_width,
            self.bar_gap,
            self.title,
            self.show_title,
        ));

        s.push_str(&r#"
        function update(svg) {
            // which of the series labels is longest?
            var maxSeriesLabelLength = 0;
            var a;
            for (a = 0; a < plot.seriesLabels.length; a++) {
              var sl = plot.seriesLabels[a];
              if (sl.length > maxSeriesLabelLength) { maxSeriesLabelLength = sl.length; }
            }
    
            // how many series are there?
            var numSeries = plot.data.length;
    
            var plotLeftMargin = 70.0;
            var plotRightMargin = plot.drawLegend ? 65.0 + maxSeriesLabelLength * 7 : 50.0;
            var plotBottomMargin = 70.0;
            var plotTopMargin = 40.0;
            var plotWidth = plot.width - plotLeftMargin - plotRightMargin;
            // var plotHeight = plot.height - plotBottomMargin - plotTopMargin;
            var height = numSeries * plot.barWidth + (numSeries - 1) * plot.barGap + plot.barWidth + plotBottomMargin + plotTopMargin;
            var plotHeight = height - plotBottomMargin - plotTopMargin;
            var originX = plotLeftMargin;
            var originY = plotTopMargin + plotHeight;
            var tickLen = 8.0;
            var minorTickLen = tickLen * 0.65;
    
            // If there are no series labels, treat it as one series.
            if (plot.seriesLabels.length === 0) {
                        plot.drawLegend = false;
                    }
    
            // colors
            var lineColor = '#47a3ff'; //'#377eb8'; // '#729ece'; // '#1f77b4'; //'#47a3ff'; //'rgb(2,145,205)';
            var highlightColor = '#ff7f00';
            var btnColor = 'rgb(170,170,170)';
            var btnHoverColor = 'rgb(150,150,150)';
            var plotBackgroundColor = 'rgb(255,255,255)';
            if (plot.drawGreyBackground) {
              plotBackgroundColor = '#CCC';
            }
            var chartBackgroundColor = 'white';
            // var gridLineColor = 'rgb(120,120,120)';
            // if (plot.drawGreyBackground) {
              var gridLineColor = '#EEE';
            // }
            var trendlineColor = 'DimGray';
            if (plot.drawGreyBackground) {
              trendlineColor = 'DimGray';
            }
            var showValueClr = "black";
            // if (plot.drawGreyBackground) {
            //   showValueClr = '#FFF';
            // }
    
            // Gridlines
            // var gridlineDash = '1, 5';
            // if (plot.drawGreyBackground) {
              var gridlineDash = 'none';
            // }
    
            var tableau20 = [[31, 119, 180], [255, 127, 14],
                 [44, 160, 44], [214, 39, 40], [148, 103, 189], [140, 86, 75],
                 [227, 119, 194], [127, 127, 127], [188, 189, 34], [23, 190, 207], 
                 [140, 86, 75], [196, 156, 148], [227, 119, 194], [247, 182, 210], 
                 [127, 127, 127], [199, 199, 199], [188, 189, 34], [219, 219, 141], 
                 [23, 190, 207], [158, 218, 229]
              ];
    
            var regularOpacity = 1.0;
            var deselectedOpacity = 0.10;
    
    
            // create the svg element
            var svgns = "http://www.w3.org/2000/svg";
            if (svg == null) {
              svg = document.createElementNS(svgns, "svg");
            } else {
              while (svg.lastChild) {
                  svg.removeChild(svg.lastChild);
              }
            }
            svg.setAttribute('width', `${plot.width}`);
            svg.setAttribute('height', `${height}`);
            var div = document.getElementById(plot.parentId);
            if (div != null) {
              div.appendChild(svg);
            } else {
              // add it to the body of the document
              document.querySelector("body").appendChild(svg);
            }
    
            // style
            var style = document.createElement("style");
            let styleString = `
            text {
              font-family:Sans,Arial;
            }
            .axisLabel {
              font-weight: normal;
            }
            .plotTitle {
                font-weight: bold;
            }
            .xTickLabel {
              fill: black;
              font-size: 85%;
              font-weight: lighter;
            }
            .yTickLabel {
              fill: black;
              font-size: 85%;
              font-weight: lighter;
            }
            .gridLine {
              stroke: ${gridLineColor};
              stroke-dasharray: ${gridlineDash};
              stroke-width: 0.8;
            }
            .tick {
              stroke: black;
              stroke-width: 0.5;
            }
            #plotBorder {
              fill: none;
              stroke: black;
              stroke-width: 0.5;
            }
            #showValue {
              font-size: 85%;
              fill: ${showValueClr};
            }
            #context-menu {
              position:absolute;
              display:none;
            }
            #context-menu ul {
              list-style:none;
              margin:0;
              padding:0;
              background: #EFEFEF;
              opacity: 0.90;
            }
            #context-menu {
              border:solid 1px #CCC;
            }
            #context-menu li {
              font-family:Sans,Arial;
              font-size: 75%;
              text-align: left;
              color:#000;
              display:block;
              padding:5px 15px;
              border-bottom:solid 1px #CCC;
            }
            #context-menu li:last-child {
              border:none;
            }
            #context-menu li:hover {
              background:#007AFF;
              color:#FFF;
            }
            `;
    
            var s;
            for (s = 0; s < numSeries; s++) {
              var clrNum = s % tableau20.length;
              if (plot.seriesLabels.length === 0) {
                // If there are no series labels, treat it as one series.
                            clrNum = 0;
                        }
              let clr = `rgb(${tableau20[clrNum][0]},${tableau20[clrNum][1]},${tableau20[clrNum][2]})`;
    
              styleString += `
              .seriesLine${s} {
                fill: none;
                stroke-width:1;
                stroke: ${clr};
                opacity: ${regularOpacity};
              }
              .seriesLine${s}:hover {
                fill: none;
                stroke-width:2;
                stroke: ${clr};
                opacity: ${regularOpacity};
              }
              .seriesLineThick${s} {
                fill: none;
                stroke-width:2;
                stroke: ${clr};
                opacity: ${regularOpacity};
              }
              .seriesLineThick${s}:hover {
                fill: none;
                stroke-width:3;
                stroke: ${clr};
                opacity: ${regularOpacity};
              }
              .bin${s} {
                fill: ${clr}; //rgb(2,145,205); // rgb(3,169,244); //#1f77b4; //rgb(0,140,190);
                stroke-width:1;
                stroke: black;
                opacity:1.0;
              }
              .median {
                fill: black;
                stroke-width:1;
                stroke: black;
                opacity:1.0;
              }
              `;
            }
            style.innerHTML = styleString;
            svg.appendChild(style);
            svg.id = "plotSvg";
    
            // background
            var background = document.createElementNS(svgns, "rect");
            background.setAttribute('width', plot.width);
            background.setAttribute('height', height);
            background.style.fill = chartBackgroundColor;
            svg.appendChild(background);
    
            // translate the origin point
            var g = document.createElementNS(svgns, "g");
            g.setAttribute('id', 'transform');
            g.setAttribute('transform', `translate(${originX},${originY})`);
            svg.appendChild(g);
    
            // plot background
            var plotBackground = document.createElementNS(svgns, "rect");
            plotBackground.setAttribute('id', 'plotBackground');
            plotBackground.setAttribute('x', 0.0);
            plotBackground.setAttribute('y', -plotHeight);
            plotBackground.setAttribute('width', plotWidth);
            plotBackground.setAttribute('height', plotHeight);
            plotBackground.style.fill = plotBackgroundColor;
            plotBackground.style.stroke = "none";
            g.appendChild(plotBackground);
    
            // what are the min/max values?
            var xMin = Infinity;
            var xMax = -Infinity;
            var val = 0;
            var maxNumPoints = 0;
            var totalNumPoints = 0;
            for (s = 0; s < numSeries; s++) {
              val = plot.data[s][0];
              if (val < xMin) { xMin = val; }
              val = plot.data[s][4];
              if (val > xMax) { xMax = val; }
            }
    
            // We don't want a data point to fall on the plot border
            xMin -= 0.0000001;
            xMax += 0.0000001;
    
            // X axis
            var xAxisTickSpacing = 0.0000001;
            var xAxisNumTicks = 1000;
            var xRange = xMax - xMin;
            a = 0;
            while (xAxisNumTicks > 20) {
              if (a % 2 == 0) {
                xAxisTickSpacing *= 5.0;
              } else {
                xAxisTickSpacing *= 2.0;
              }
              a++;
              xAxisNumTicks = Math.ceil(xRange / xAxisTickSpacing);
            }
            xMin = Math.floor(xMin / xAxisTickSpacing) * xAxisTickSpacing;
            xAxisNumTicks = Math.ceil((xMax - xMin) / xAxisTickSpacing);
            xRange = xAxisTickSpacing * xAxisNumTicks;
            var xSigDigits = Math.max(decimalPlaces(xMin+xAxisTickSpacing), decimalPlaces(xMin+2*xAxisTickSpacing)); //Math.round(0.1 / xAxisTickSpacing);
            if (xSigDigits === 0 && (xMax - xMin) < xAxisNumTicks) { xSigDigits += 1; }
            var dominantTick = 0;
            if (decimalPlaces(xMin) > decimalPlaces(xMin+xAxisTickSpacing)) {
              dominantTick = 1;
            }
            for (a = 0; a <= xAxisNumTicks; a++) {
                // grid line
                if (plot.drawGridlines) {
                  if (xAxisNumTicks <= 10 || a % 2 == dominantTick) {
                    if (a > 0 && a < xAxisNumTicks) {
                      var line = document.createElementNS(svgns, "line");
                      line.setAttribute('x1', (a * xAxisTickSpacing) / xRange * plotWidth);
                      line.setAttribute('y1', 0);
                      line.setAttribute('x2', (a * xAxisTickSpacing) / xRange * plotWidth);
                      line.setAttribute('y2', -plotHeight);
                      line.setAttribute('class', 'gridLine');
                      g.appendChild(line);
                    }
                  }
                }
                // tick mark
                var line = document.createElementNS(svgns, "line");
                line.setAttribute('x1', (a * xAxisTickSpacing) / xRange * plotWidth);
                line.setAttribute('y1', 0);
                line.setAttribute('x2', (a * xAxisTickSpacing) / xRange * plotWidth);
                if (xAxisNumTicks <= 10 || a % 2 == dominantTick) {
                  line.setAttribute('y2', tickLen);
                } else {
                  line.setAttribute('y2', minorTickLen);
                }
                line.setAttribute('class', 'tick');
                g.appendChild(line);
    
                // labels
                if (xAxisNumTicks <= 10 || a % 2 == dominantTick) {
                    // tick label
                    var label = document.createElementNS(svgns, "text");
                    label.setAttribute('x', (a * xAxisTickSpacing) / xRange * plotWidth);
                    label.setAttribute('y', tickLen + 5);
                    label.setAttribute('text-anchor', 'middle');
                    label.setAttribute('dominant-baseline', 'hanging');
                    label.setAttribute('class', 'xTickLabel');
                    label.innerHTML = `${(xMin + a * xAxisTickSpacing).toFixed(xSigDigits)}`;
                    g.appendChild(label);
                }
            }
    
            // axis labels
            var xLabel = document.createElementNS(svgns, "text");
            xLabel.setAttribute('x', plotWidth / 2.0);
            xLabel.setAttribute('y', tickLen + 25.0);
            xLabel.setAttribute('text-anchor', 'middle');
            xLabel.setAttribute('dominant-baseline', 'hanging');
            xLabel.setAttribute('class', 'axisLabel');
            xLabel.innerHTML = plot.xAxisLabel;
            g.appendChild(xLabel);

            // title
            if (plot.showTitle) {
              var xLabel = document.createElementNS(svgns, "text");
              xLabel.setAttribute('x', plotWidth / 2.0);
              xLabel.setAttribute('y', -plotHeight - 25);
              xLabel.setAttribute('text-anchor', 'middle');
              xLabel.setAttribute('dominant-baseline', 'hanging');
              xLabel.setAttribute('class', 'plotTitle');
              xLabel.innerHTML = plot.title;
              g.appendChild(xLabel);
            }
    
            // text to show values when hover over
            var showValue = document.createElementNS(svgns, "text");
            showValue.setAttribute('x', 10);
            showValue.setAttribute('text-anchor', 'start');
    
            // draw the line(s)
            var g2 = document.createElementNS(svgns, "g");
            g2.setAttribute('id', 'lines');
            g.appendChild(g2);
    
            for (let s = 0; s < numSeries; s++) {
              let order = numSeries - s;
              var seriesLabel = "seriesLine";
              var line = document.createElementNS(svgns, "line");
              line.setAttribute('x1', (plot.data[s][0] - xMin) / xRange * plotWidth);
              line.setAttribute('y1', -(plot.barWidth + plot.barGap) * order + plot.barGap);
              line.setAttribute('x2', (plot.data[s][4] - xMin) / xRange * plotWidth);
              line.setAttribute('y2', -(plot.barWidth + plot.barGap) * order  + plot.barGap);
              line.setAttribute('class', 'median');
              g2.appendChild(line);
    
              var r = document.createElementNS(svgns, "rect");
              r.setAttribute('x', (plot.data[s][1] - xMin) / xRange * plotWidth);
              r.setAttribute('y', (-(plot.barWidth + plot.barGap) * order) - plot.barWidth/2.0 + plot.barGap);
              r.setAttribute('width', (plot.data[s][3] - xMin) / xRange * plotWidth - (plot.data[s][1] - xMin) / xRange * plotWidth);
              r.setAttribute('height', plot.barWidth);
              r.setAttribute('class', `bin${s}`);
              g2.appendChild(r);
    
              var median = document.createElementNS(svgns, "line");
              median.setAttribute('x1', (plot.data[s][2] - xMin) / xRange * plotWidth);
              median.setAttribute('y1', -(plot.barWidth + plot.barGap) * order - plot.barWidth/2.0 + plot.barGap);
              median.setAttribute('x2', (plot.data[s][2] - xMin) / xRange * plotWidth);
              median.setAttribute('y2', -(plot.barWidth + plot.barGap) * order + plot.barWidth/2.0 + plot.barGap);
              median.setAttribute('class', 'median');
              g2.appendChild(median);
    
              var median = document.createElementNS(svgns, "line");
              median.setAttribute('x1', (plot.data[s][0] - xMin) / xRange * plotWidth);
              median.setAttribute('y1', -(plot.barWidth + plot.barGap) * order - plot.barWidth/4.0 + plot.barGap);
              median.setAttribute('x2', (plot.data[s][0] - xMin) / xRange * plotWidth);
              median.setAttribute('y2', -(plot.barWidth + plot.barGap) * order + plot.barWidth/4.0 + plot.barGap);
              median.setAttribute('class', 'median');
              g2.appendChild(median);
    
              var median = document.createElementNS(svgns, "line");
              median.setAttribute('x1', (plot.data[s][4] - xMin) / xRange * plotWidth);
              median.setAttribute('y1', -(plot.barWidth + plot.barGap) * order - plot.barWidth/4.0 + plot.barGap);
              median.setAttribute('x2', (plot.data[s][4] - xMin) / xRange * plotWidth);
              median.setAttribute('y2', -(plot.barWidth + plot.barGap) * order + plot.barWidth/4.0 + plot.barGap);
              median.setAttribute('class', 'median');
              g2.appendChild(median);
            }
    
            // Show value label
            showValue.setAttribute('id', 'showValue');
            showValue.setAttribute('y', -plotHeight + 20);
            g.appendChild(showValue);
    
            // plot border
            var plotBorder = document.createElementNS(svgns, "rect");
            plotBorder.setAttribute('x', 0);
            plotBorder.setAttribute('y', -plotHeight);
            plotBorder.setAttribute('width', plotWidth);
            plotBorder.setAttribute('height', plotHeight);
            plotBorder.id = "plotBorder";
            g.appendChild(plotBorder);
    
            // add a legend
            if (plot.seriesLabels.length > 0 && plot.drawLegend) {
              var legend = document.createElementNS(svgns, "g");
              legend.setAttribute('id', 'legend');
              g.appendChild(legend);
              for (let s = 0; s < numSeries; s++) {
                var y = -(plotHeight + 15 - 23 * (s+1));
                var line = document.createElementNS(svgns, "rect");
                line.setAttribute('x', plotWidth + 10);
                line.setAttribute('y', y-4);
                line.setAttribute('width', 30);
                line.setAttribute('height', 8);
                line.setAttribute('class', `bin${s}`);
                legend.appendChild(line);
    
                var legendLabel = document.createElementNS(svgns, "text");
                legendLabel.setAttribute('x', plotWidth + 48);
                legendLabel.setAttribute('y', y);
                legendLabel.setAttribute('text-anchor', 'left');
                legendLabel.setAttribute('dominant-baseline', 'middle');
                legendLabel.setAttribute('class', 'xTickLabel');
                legendLabel.innerHTML = plot.seriesLabels[s];
                legend.appendChild(legendLabel);
              }
            }
    
            // Add an invisible context menu to the parentId.
            var cm = document.createElement('div');
            cm.id = 'context-menu';
            cm.className = 'context-menu';
            var list = document.createElement('ul');
    
            var copyBtn = document.createElement("li");
            copyBtn.innerHTML = "Copy";
            copyBtn.addEventListener('click', function() {
              var content = `<svg xmlns='${svgns}' width='${plot.width}' height='${plot.height}'>\n${svg.innerHTML}\n</svg>`;
              // Create an auxiliary hidden input
              var aux = document.createElement("input");
              // Get the text from the element passed into the input
              aux.setAttribute("value", content);
              // Append the aux input to the body
              document.body.appendChild(aux);
              // Highlight the content
              aux.select();
              // Execute the copy command
              document.execCommand("copy");
              // Remove the input from the body
              document.body.removeChild(aux);
              // Give a notification
              alert("The plot's SVG content has been copied to the clipboard.");
            }, false);
            list.appendChild(copyBtn);
    
            var gridlineBtn = document.createElement("li");
            var verb = plot.drawGridlines ? "Hide " : "Show ";
            gridlineBtn.innerHTML = verb + "Gridlines";
            gridlineBtn.addEventListener('click', function() {
              plot.drawGridlines = !plot.drawGridlines;
              // update the context menu label
              var verb = plot.drawGridlines ? "Hide " : "Show ";
              gridlineBtn.innerHTML = verb + "Gridlines";
              update(svg);
            }, false);
            list.appendChild(gridlineBtn);
    
            if (plot.seriesLabels.length > 0) {
              var legendBtn = document.createElement("li");
              var verb = plot.drawLegend ? "Hide " : "Show ";
              legendBtn.innerHTML = verb + "Legend";
              legendBtn.addEventListener('click', function() {
                plot.drawLegend = !plot.drawLegend;
                // update the context menu label
                var verb = plot.drawLegend ? "Hide " : "Show ";
                legendBtn.innerHTML = verb + "Legend";
                update(svg);
              }, false);
              list.appendChild(legendBtn);
            }
    
            var backgroundColorBtn = document.createElement("li");
            var verb = plot.drawGreyBackground ? "Light " : "Dark ";
            backgroundColorBtn.innerHTML = verb + "Background";
            backgroundColorBtn.addEventListener('click', function() {
              plot.drawGreyBackground = !plot.drawGreyBackground;
              // update the context menu label
              var verb = plot.drawGreyBackground ? "Light " : "Dark ";
              backgroundColorBtn.innerHTML = verb + "Background";
              update(svg);
            }, false);
            list.appendChild(backgroundColorBtn);
    
            cm.appendChild(list);
            document.getElementById(plot.parentId).appendChild(cm);
    
            var menu = document.getElementById('context-menu');
            document.onclick = function () {
                menu.style.display = 'none';
            };
    
            document.getElementById('plotSvg').oncontextmenu = function (evt) {
                evt = (evt) ? evt : ((event) ? event : null);
                var posnX = (evt.pageX) ? evt.pageX : ((evt.offsetX) ? evt.offsetX + 10 : null);
                var posnY = (evt.pageY) ? evt.pageY : ((evt.offsetY) ? evt.offsetY + 10 : null);
                menu.style.left = posnX + 'px';
                menu.style.top = posnY + 'px';
                menu.style.display = 'block';
                if (typeof evt.preventDefault != "undefined") {
                    evt.preventDefault();
                } else {
                    evt.returnValue = false;
                }
            };
          }
    
          function decimalPlaces(num) {
            var match = (''+num).match(/(?:\.(\d+))?(?:[eE]([+-]?\d+))?$/);
            if (!match) { return 0; }
            return Math.max(
                 0,
                 // Number of digits right of decimal point.
                 (match[1] ? match[1].length : 0)
                 // Adjust for scientific notation.
                 - (match[2] ? +match[2] : 0));
          }
    
          update(null);
        </script>
    "#);

        s
    }
}
